#!/usr/bin/env python3
"""
Stage 55: Multi-Step Curriculum Training Pipeline for ProofBDH.

Key Objectives:
1. Multi-Step Supervision: Trains on intermediate state-action pairs and goal transitions.
2. Prevents Overfitting: Monitors held-out validation loss with strict early stopping (patience=3 epochs).
3. Numerical Stability: Gradient clipping (norm <= 0.5) and FP32/BF16 scaling to eliminate loss spikes.
4. Parameter Bound: Enforces strict <= 10,000,000 parameter budget (9,938,560 parameters).
"""

import os
os.environ.setdefault("PYTORCH_CUDA_ALLOC_CONF", "expandable_segments:True")
import sys
import json
import time
import math
import random
import argparse
from pathlib import Path
from typing import List, Dict, Any, Tuple, Optional

ROOT_DIR = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT_DIR))
sys.path.insert(0, str(ROOT_DIR / "scripts" / "ml"))

import torch
import torch.nn as nn
from torch.utils.data import Dataset, DataLoader
from torch.optim import AdamW
from torch.optim.lr_scheduler import CosineAnnealingLR

from proof_tokenizer import ProofTokenizer
from proof_bdh import ProofBDH, BDHConfig

DATA_DIR = ROOT_DIR / "data"
TRACES_DIR = DATA_DIR / "clean_mathlib_traces"
CHECKPOINT_DIR = DATA_DIR / "native_prover" / "checkpoints"
CHECKPOINT_DIR.mkdir(parents=True, exist_ok=True)

VOCAB_PATH = DATA_DIR / "native_prover" / "vocab.json"
BEST_CHECKPOINT = CHECKPOINT_DIR / "bdh_9m_curriculum_best.pt"
PROGRESS_FILE = CHECKPOINT_DIR / "curriculum_training_progress.json"


class ProofDataset(Dataset):
    """Memory-efficient PyTorch Dataset for tokenized proof traces."""

    def __init__(self, data: List[List[int]], max_len: int = 512, pad_id: int = 0):
        self.data = data
        self.max_len = max_len
        self.pad_id = pad_id

    def __len__(self) -> int:
        return len(self.data)

    def __getitem__(self, idx: int) -> Tuple[List[int], List[int]]:
        tokens = self.data[idx]
        if len(tokens) > self.max_len:
            tokens = tokens[:self.max_len]
        return tokens[:-1], tokens[1:]


def collate_proof_batch(batch, max_len: int = 512, pad_id: int = 0) -> Tuple[torch.Tensor, torch.Tensor]:
    """Dynamically pads batch to the longest sample in the batch rather than static max_seq_len."""
    batch_max = min(max_len - 1, max(len(ids) for ids, tgts in batch))
    padded_x, padded_y = [], []
    for ids, tgts in batch:
        cur_x = ids[:batch_max]
        cur_y = tgts[:batch_max]
        p_len = batch_max - len(cur_x)
        padded_x.append(cur_x + [pad_id] * p_len)
        padded_y.append(cur_y + [-100] * p_len)
    return torch.tensor(padded_x, dtype=torch.long), torch.tensor(padded_y, dtype=torch.long)


def load_and_tokenize(
    file_paths: List[Path],
    tokenizer: ProofTokenizer,
    max_samples: Optional[int] = None
) -> List[List[int]]:
    """Loads JSONL files and tokenizes proof text."""
    tokenized = []
    seen = set()

    for path in file_paths:
        if not path.exists():
            continue
        with open(path, "r", encoding="utf-8") as f:
            for line in f:
                if not line.strip():
                    continue
                try:
                    record = json.loads(line)
                    text = record.get("text") or record.get("formatted_trace") or ""
                    if not text or text in seen:
                        continue
                    seen.add(text)
                    ids = tokenizer.encode(text, add_bos=True, add_eos=True)
                    if len(ids) >= 8:
                        tokenized.append(ids)
                    if max_samples and len(tokenized) >= max_samples:
                        break
                except Exception:
                    continue
        if max_samples and len(tokenized) >= max_samples:
            break

    random.shuffle(tokenized)
    return tokenized


def evaluate_val_loss(
    model: ProofBDH,
    val_loader: DataLoader,
    device: torch.device,
    use_amp: bool = False,
    amp_dtype: torch.dtype = torch.float16
) -> float:
    """Computes cross-entropy loss over validation dataset."""
    model.eval()
    total_loss = 0.0
    total_tokens = 0

    criterion = nn.CrossEntropyLoss(ignore_index=-100, reduction="sum")

    with torch.no_grad():
        for input_ids, targets in val_loader:
            input_ids = input_ids.to(device)
            targets = targets.to(device)

            with torch.amp.autocast('cuda', dtype=amp_dtype, enabled=use_amp):
                out = model(input_ids)
                logits = out[0] if isinstance(out, tuple) else out
                loss = criterion(logits.view(-1, logits.size(-1)), targets.view(-1))

            valid_mask = (targets != -100)
            num_valid = valid_mask.sum().item()

            total_loss += loss.item()
            total_tokens += num_valid

    model.train()
    return total_loss / max(1, total_tokens)


def train_curriculum(
    tier: str = "10m",
    epochs: int = 6,
    batch_size: Optional[int] = None,
    grad_accum_steps: int = 1,
    lr: float = 2.0e-4,
    patience: int = 3,
    max_seq_len: int = 512,
    device_name: Optional[str] = None
):
    tier = tier.lower().strip()
    if batch_size is None:
        batch_size = 2 if tier == "95m" else 16
    if tier == "95m" and grad_accum_steps == 1:
        grad_accum_steps = 4

    if device_name is None:
        device_name = "cuda" if torch.cuda.is_available() else "cpu"
    device = torch.device(device_name)

    print("=" * 80)
    print(f"STAGE 55: MULTI-STEP CURRICULUM TRAINING FOR PROOFBDH [TIER: {tier.upper()}]")
    effective_bsz = batch_size * grad_accum_steps
    print(f"Device: {device} | Micro-Batch: {batch_size} | Grad Accum: {grad_accum_steps} (Effective: {effective_bsz}) | LR: {lr}")
    print(f"Max Seq Len: {max_seq_len} | Patience: {patience}")
    print("=" * 80)

    tokenizer = ProofTokenizer(str(VOCAB_PATH))
    print(f"Loaded tokenizer with vocab size: {len(tokenizer.vocab)}")

    # 1. Load Training and Validation Datasets
    balanced_file = TRACES_DIR / "stage59_balanced_train.jsonl"
    expanded_file = TRACES_DIR / "stage57_expanded_train.jsonl"
    if tier == "95m":
        if balanced_file.exists():
            train_files = [balanced_file]
            print(f"Using Stage 59 Balanced Training Corpus: {balanced_file.name}")
        elif expanded_file.exists():
            train_files = [expanded_file]
            print(f"Using Stage 57 Expanded Training Corpus: {expanded_file.name}")
        else:
            train_files = [
                TRACES_DIR / "multistep_train_traces.jsonl",
                TRACES_DIR / "stage51_unified_train.jsonl"
            ]
    else:
        train_files = [
            TRACES_DIR / "multistep_train_traces.jsonl",
            TRACES_DIR / "stage51_unified_train.jsonl"
        ]
    val_files = [
        TRACES_DIR / "val_traces.jsonl"
    ]

    print("Loading and tokenizing training datasets...")
    train_tokens = load_and_tokenize(train_files, tokenizer)
    val_tokens = load_and_tokenize(val_files, tokenizer, max_samples=1000)

    print(f"Train samples: {len(train_tokens):,} | Val samples: {len(val_tokens):,}")

    pad_id = tokenizer.vocab.get("<pad>", 0)
    train_dataset = ProofDataset(train_tokens, max_len=max_seq_len, pad_id=pad_id)
    val_dataset = ProofDataset(val_tokens, max_len=max_seq_len, pad_id=pad_id)

    collate = lambda b: collate_proof_batch(b, max_len=max_seq_len, pad_id=pad_id)
    train_loader = DataLoader(train_dataset, batch_size=batch_size, shuffle=True, drop_last=True, collate_fn=collate)
    val_loader = DataLoader(val_dataset, batch_size=batch_size, shuffle=False, collate_fn=collate)

    # 2. Tier Checkpoint and Parameter Resolution
    if tier == "95m":
        best_checkpoint = CHECKPOINT_DIR / "bdh_95m_curriculum_best.pt"
        progress_file = CHECKPOINT_DIR / "curriculum_95m_progress.json"
        max_params = 100_000_000
        if best_checkpoint.exists():
            print(f"Resuming from existing BDH-95M checkpoint: {best_checkpoint.name}")
            model = ProofBDH.load_checkpoint(str(best_checkpoint), device=device_name)
        else:
            print("Initializing fresh BDH-95M architecture (dim=768, n_layers=10, n_heads=12, hidden_dim=3072)...")
            cfg = BDHConfig.from_preset("95m", vocab_size=len(tokenizer.vocab), max_seq_len=max_seq_len)
            model = ProofBDH(cfg)
    else:
        best_checkpoint = CHECKPOINT_DIR / "bdh_9m_curriculum_best.pt"
        progress_file = CHECKPOINT_DIR / "curriculum_training_progress.json"
        max_params = 10_000_000
        stage52_ckpt = CHECKPOINT_DIR / "bdh_9m_stage52_best.pt"
        if stage52_ckpt.exists():
            print(f"Initializing from Stage 52 checkpoint: {stage52_ckpt.name}")
            model = ProofBDH.load_checkpoint(str(stage52_ckpt), device=device_name)
        else:
            cfg = BDHConfig.from_preset("10m", vocab_size=len(tokenizer.vocab), max_seq_len=max_seq_len)
            model = ProofBDH(cfg)

    model.to(device)
    param_count = model.count_parameters()
    print(f"Total Parameters: {param_count:,} (Constraint <= {max_params:,}: {param_count <= max_params})")
    assert param_count <= max_params, f"Parameter budget exceeded: {param_count} > {max_params}"

    # 3. Setup Optimization
    optimizer = AdamW(model.parameters(), lr=lr, weight_decay=0.01, betas=(0.9, 0.98))
    scheduler = CosineAnnealingLR(optimizer, T_max=epochs * (len(train_loader) // grad_accum_steps), eta_min=1.0e-5)
    criterion = nn.CrossEntropyLoss(ignore_index=-100)

    use_amp = (device.type == "cuda")
    amp_dtype = torch.bfloat16 if (use_amp and torch.cuda.is_bf16_supported()) else torch.float16

    best_val_loss = float("inf")
    epochs_no_improve = 0
    t0 = time.time()

    # Initial Validation Evaluation
    init_val_loss = evaluate_val_loss(model, val_loader, device, use_amp=use_amp, amp_dtype=amp_dtype)
    best_val_loss = init_val_loss
    print(f"Baseline Validation Loss: {init_val_loss:.4f} (Perplexity: {math.exp(min(20, init_val_loss)):.2f})\n")

    # 4. Training Loop
    for epoch in range(1, epochs + 1):
        model.train()
        total_epoch_loss = 0.0
        steps = 0
        optimizer.zero_grad()

        for step, (input_ids, targets) in enumerate(train_loader, 1):
            input_ids = input_ids.to(device)
            targets = targets.to(device)

            with torch.amp.autocast('cuda', dtype=amp_dtype, enabled=use_amp):
                out = model(input_ids)
                logits = out[0] if isinstance(out, tuple) else out
                loss = criterion(logits.view(-1, logits.size(-1)), targets.view(-1))
                loss_scaled = loss / grad_accum_steps

            if torch.isnan(loss) or torch.isinf(loss):
                print(f"[WARN] Non-finite loss encountered at epoch {epoch}, step {step}. Skipping.")
                continue

            loss_scaled.backward()

            if step % grad_accum_steps == 0 or step == len(train_loader):
                torch.nn.utils.clip_grad_norm_(model.parameters(), max_norm=0.5)
                optimizer.step()
                optimizer.zero_grad()
                scheduler.step()

            total_epoch_loss += loss.item()
            steps += 1

            if step % (50 * grad_accum_steps) == 0 or step == len(train_loader):
                curr_lr = scheduler.get_last_lr()[0]
                print(f"Epoch {epoch:02d}/{epochs:02d} | Step {step:04d}/{len(train_loader)} | Train Loss: {loss.item():.4f} | LR: {curr_lr:.2e}", flush=True)
                if device.type == "cuda":
                    torch.cuda.empty_cache()

        # Validation Check at End of Epoch
        val_loss = evaluate_val_loss(model, val_loader, device, use_amp=use_amp, amp_dtype=amp_dtype)
        val_ppl = math.exp(min(20, val_loss))
        avg_train_loss = total_epoch_loss / max(1, steps)

        print(f"\n---> Epoch {epoch:02d} Summary: Train Loss: {avg_train_loss:.4f} | Val Loss: {val_loss:.4f} (PPL: {val_ppl:.2f})", flush=True)

        # Early Stopping Logic
        if val_loss < best_val_loss - 1e-4:
            print(f"     [NEW BEST] Validation loss improved from {best_val_loss:.4f} to {val_loss:.4f}. Saving checkpoint to {best_checkpoint.name}.", flush=True)
            best_val_loss = val_loss
            epochs_no_improve = 0
            model.save_checkpoint(str(best_checkpoint))
        else:
            epochs_no_improve += 1
            print(f"     [PATIENCE] No improvement for {epochs_no_improve}/{patience} epoch(s).", flush=True)
            if epochs_no_improve >= patience:
                print(f"\n[EARLY STOPPING] Validation loss stopped improving. Halting training to prevent overfitting.", flush=True)
                break

        # Record Progress
        progress = {
            "tier": tier,
            "epoch": epoch,
            "best_val_loss": round(best_val_loss, 4),
            "last_val_loss": round(val_loss, 4),
            "parameter_count": param_count,
            "elapsed_minutes": round((time.time() - t0) / 60.0, 2),
            "last_updated": time.strftime("%Y-%m-%d %H:%M:%S")
        }
        with open(progress_file, "w", encoding="utf-8") as f:
            json.dump(progress, f, indent=2)

    print(f"\nTraining completed in {(time.time() - t0) / 60.0:.1f} minutes. Best Val Loss: {best_val_loss:.4f}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Multi-Step Curriculum Trainer for ProofBDH")
    parser.add_argument("--tier", type=str, default="10m", choices=["10m", "95m"], help="Model tier: 10m baseline or 95m scale")
    parser.add_argument("--epochs", type=int, default=6)
    parser.add_argument("--batch-size", type=int, default=None, help="Batch size (defaults to 16 for 10m, 4 for 95m)")
    parser.add_argument("--grad-accum", type=int, default=1, help="Gradient accumulation steps")
    parser.add_argument("--lr", type=float, default=2.0e-4)
    parser.add_argument("--patience", type=int, default=3)
    parser.add_argument("--max-seq-len", type=int, default=512)
    parser.add_argument("--device", type=str, default=None)
    args = parser.parse_args()

    train_curriculum(
        tier=args.tier,
        epochs=args.epochs,
        batch_size=args.batch_size,
        grad_accum_steps=args.grad_accum,
        lr=args.lr,
        patience=args.patience,
        max_seq_len=args.max_seq_len,
        device_name=args.device
    )
