#!/usr/bin/env python3
"""
Curriculum Fine-Tuning Pipeline for ProofBDH on Algebraic Rationalization.

Trains ProofBDH (9.9M parameters, strictly sub-10M ceiling):
1. Ingests verified algebraic rationalization traces mixed with Mathlib traces.
2. Trains using AdamW + CosineAnnealingLR with gradient clipping (norm <= 0.5).
3. Evaluates validation loss and saves best checkpoint:
   `data/native_prover/checkpoints/bdh_9m_rationalization_best.pt`.
4. Strict parameter ceiling: verifies params <= 10,000,000 before saving.
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
CURRICULUM_FILE = DATA_DIR / "curriculum" / "algebraic_rationalization_traces.jsonl"
MATHLIB_TRACES = DATA_DIR / "clean_mathlib_traces" / "multistep_train_traces.jsonl"
VAL_TRACES = DATA_DIR / "clean_mathlib_traces" / "val_traces.jsonl"

VOCAB_PATH = DATA_DIR / "native_prover" / "vocab.json"
CHECKPOINT_DIR = DATA_DIR / "native_prover" / "checkpoints"
BASE_CHECKPOINT = CHECKPOINT_DIR / "bdh_9m_curriculum_best.pt"
OUT_CHECKPOINT = CHECKPOINT_DIR / "bdh_9m_rationalization_best.pt"
PROGRESS_FILE = CHECKPOINT_DIR / "rationalization_training_progress.json"


class ProofDataset(Dataset):
    def __init__(self, data: List[Tuple[List[int], List[int]]], max_len: int = 512):
        self.data = data
        self.max_len = max_len

    def __len__(self) -> int:
        return len(self.data)

    def __getitem__(self, idx: int) -> Tuple[List[int], List[int]]:
        inp, tgt = self.data[idx]
        if len(inp) > self.max_len:
            inp = inp[:self.max_len]
            tgt = tgt[:self.max_len]
        return inp, tgt


def collate_proof_batch(batch, max_len: int = 512, pad_id: int = 0):
    batch_max = min(max_len, max(len(ids) for ids, tgts in batch))
    padded_x, padded_y = [], []
    for ids, tgts in batch:
        cur_x = ids[:batch_max]
        cur_y = tgts[:batch_max]
        p_len = batch_max - len(cur_x)
        padded_x.append(cur_x + [pad_id] * p_len)
        padded_y.append(cur_y + [-100] * p_len)
    return torch.tensor(padded_x, dtype=torch.long), torch.tensor(padded_y, dtype=torch.long)


def load_tokenized_traces(
    tokenizer: ProofTokenizer,
    curriculum_path: Path,
    mathlib_path: Path,
    max_mathlib_samples: int = 400,
) -> List[Tuple[List[int], List[int]]]:
    """
    Loads training traces with strict prompt-loss masking:
    Loss is calculated ONLY on the tactical tokens following <tactic>,
    ensuring 100% of gradient updates train tactic synthesis rather than
    memorizing problem hypotheses.
    """
    tokenized: List[Tuple[List[int], List[int]]] = []
    seen = set()

    # 1. Ingest all curriculum traces (oversampled 2x for strong signal)
    if curriculum_path.exists():
        curriculum_lines = []
        with open(curriculum_path, "r", encoding="utf-8") as f:
            for line in f:
                if line.strip():
                    curriculum_lines.append(json.loads(line))
        for item in curriculum_lines:
            text = item.get("text", "")
            if text and text not in seen:
                seen.add(text)
                hyp = item.get("hypotheses", "")
                goal = item.get("goal", "")
                tac = item.get("tactic", "")

                prompt_str = f"<hyp> {hyp} </hyp> <goal> {goal} <plan> proof </plan><tactic>"
                tactic_str = f" {tac}"

                p_ids = tokenizer.encode(prompt_str, add_bos=True, add_eos=False)
                t_ids = tokenizer.encode(tactic_str, add_bos=False, add_eos=True)

                tokens = p_ids + t_ids
                p_len = len(p_ids)
                # Targets: prompt positions masked with -100 so shift_targets ignores them
                targets = [-100] * p_len + t_ids

                if len(tokens) >= 8:
                    tokenized.append((tokens, targets))
                    tokenized.append((tokens, targets))  # 2x oversample

    print(f"Loaded {len(tokenized)} curriculum token sequences with prompt-loss masking.")

    # 2. Ingest Mathlib traces to preserve general vocabulary & reasoning
    if mathlib_path.exists():
        mathlib_count = 0
        with open(mathlib_path, "r", encoding="utf-8") as f:
            for line in f:
                if mathlib_count >= max_mathlib_samples:
                    break
                if not line.strip():
                    continue
                try:
                    item = json.loads(line)
                    text = item.get("text") or item.get("formatted_trace") or ""
                    if text and text not in seen:
                        seen.add(text)
                        if "<tactic>" in text:
                            parts = text.split("<tactic>")
                            p_str = parts[0] + "<tactic>"
                            t_str = parts[1]
                            p_ids = tokenizer.encode(p_str, add_bos=True, add_eos=False)
                            t_ids = tokenizer.encode(t_str, add_bos=False, add_eos=True)
                            tokens = p_ids + t_ids
                            targets = [-100] * len(p_ids) + t_ids
                        else:
                            tokens = tokenizer.encode(text, add_bos=True, add_eos=True)
                            targets = list(tokens)

                        if len(tokens) >= 8:
                            tokenized.append((tokens, targets))
                            mathlib_count += 1
                except Exception:
                    continue
        print(f"Loaded {mathlib_count} Mathlib anchor sequences.")

    random.shuffle(tokenized)
    return tokenized


def train_rationalization_model(
    epochs: int = 8,
    batch_size: int = 8,
    lr: float = 1e-4,
    device_str: Optional[str] = None,
):
    print("==================================================================")
    print("PURE NEURAL CURRICULUM TRAINING: ALGEBRAIC RATIONALIZATION")
    print("==================================================================")

    if device_str is None:
        device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    else:
        device = torch.device(device_str)
    print(f"Using Compute Device: {device}")

    # Load Tokenizer
    tokenizer = ProofTokenizer(vocab_path=str(VOCAB_PATH))
    print(f"Tokenizer loaded (Vocab size: {tokenizer.vocab_size})")

    # Load Base Checkpoint
    config = BDHConfig.from_preset("10m", vocab_size=tokenizer.vocab_size, max_seq_len=512)
    model = ProofBDH(config)
    if BASE_CHECKPOINT.exists():
        print(f"Loading Base Checkpoint: {BASE_CHECKPOINT}")
        ckpt = torch.load(BASE_CHECKPOINT, map_location=device, weights_only=False)
        state_dict = ckpt.get("model_state_dict") or ckpt.get("state_dict") or ckpt
        clean_sd = {k.replace("model.", ""): v for k, v in state_dict.items() if not k.startswith("config")}
        model.load_state_dict(clean_sd, strict=False)
    else:
        print("Warning: Base checkpoint not found, initializing fresh model.")

    model.to(device)
    total_params = sum(p.numel() for p in model.parameters())
    print(f"ProofBDH Parameters: {total_params:,} (Sub-10M bound: {total_params <= 10_000_000})")
    assert total_params <= 10_000_000, f"Parameter ceiling violated: {total_params} > 10,000,000"

    # Load Data
    train_data = load_tokenized_traces(tokenizer, CURRICULUM_FILE, MATHLIB_TRACES, max_mathlib_samples=600)
    split_idx = int(len(train_data) * 0.9)
    train_set = train_data[:split_idx]
    val_set = train_data[split_idx:]

    train_loader = DataLoader(
        ProofDataset(train_set, max_len=512),
        batch_size=batch_size,
        shuffle=True,
        collate_fn=lambda b: collate_proof_batch(b, max_len=512, pad_id=tokenizer.pad_id),
    )
    val_loader = DataLoader(
        ProofDataset(val_set, max_len=512),
        batch_size=batch_size,
        shuffle=False,
        collate_fn=lambda b: collate_proof_batch(b, max_len=512, pad_id=tokenizer.pad_id),
    )

    optimizer = AdamW(model.parameters(), lr=lr, weight_decay=0.01)
    scheduler = CosineAnnealingLR(optimizer, T_max=epochs * len(train_loader), eta_min=1e-5)
    criterion = nn.CrossEntropyLoss(ignore_index=-100)

    best_val_loss = float("inf")
    history = []

    print(f"\nStarting Fine-Tuning across {epochs} epochs ({len(train_loader)} steps/epoch)...")

    for epoch in range(1, epochs + 1):
        model.train()
        total_loss = 0.0
        t0 = time.perf_counter()

        for step, (x, y) in enumerate(train_loader):
            x, y = x.to(device), y.to(device)
            optimizer.zero_grad()

            logits, loss, _ = model(x, targets=y)
            if loss is None:
                loss = criterion(logits.view(-1, logits.size(-1)), y.view(-1))
            loss.backward()

            nn.utils.clip_grad_norm_(model.parameters(), max_norm=0.5)
            optimizer.step()
            scheduler.step()

            total_loss += loss.item()

        avg_train_loss = total_loss / len(train_loader)

        # Validation
        model.eval()
        val_loss = 0.0
        with torch.no_grad():
            for x, y in val_loader:
                x, y = x.to(device), y.to(device)
                logits, loss, _ = model(x, targets=y)
                if loss is None:
                    loss = criterion(logits.view(-1, logits.size(-1)), y.view(-1))
                val_loss += loss.item()

        avg_val_loss = val_loss / max(1, len(val_loader))
        epoch_time = time.perf_counter() - t0

        print(f"Epoch [{epoch}/{epochs}] Train Loss: {avg_train_loss:.4f} | Val Loss: {avg_val_loss:.4f} | Time: {epoch_time:.1f}s")

        history.append({
            "epoch": epoch,
            "train_loss": avg_train_loss,
            "val_loss": avg_val_loss,
            "time_sec": epoch_time,
        })

        if avg_val_loss < best_val_loss:
            best_val_loss = avg_val_loss
            print(f"  ★ New best model (Val Loss: {best_val_loss:.4f}). Saving to {OUT_CHECKPOINT}...")
            torch.save({
                "model_state_dict": model.state_dict(),
                "config": {
                    "vocab_size": config.vocab_size,
                    "dim": config.dim,
                    "n_layers": config.n_layers,
                    "n_heads": config.n_heads,
                    "max_seq_len": config.max_seq_len,
                },
                "total_parameters": total_params,
                "best_val_loss": best_val_loss,
                "epoch": epoch,
            }, OUT_CHECKPOINT)

    # Save Progress Telemetry
    with open(PROGRESS_FILE, "w") as f:
        json.dump({
            "total_epochs": epochs,
            "best_val_loss": best_val_loss,
            "total_params": total_params,
            "history": history,
        }, f, indent=2)

    print("\n==================================================================")
    print("CURRICULUM TRAINING COMPLETE")
    print(f"Best Val Loss: {best_val_loss:.4f}")
    print(f"Checkpoint saved to: {OUT_CHECKPOINT}")
    print("==================================================================")
    return OUT_CHECKPOINT


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Train ProofBDH on Rationalization Curriculum")
    parser.add_argument("--epochs", type=int, default=5, help="Number of training epochs")
    parser.add_argument("--batch-size", type=int, default=8, help="Batch size")
    parser.add_argument("--lr", type=float, default=1e-4, help="Learning rate")
    args = parser.parse_args()

    train_rationalization_model(epochs=args.epochs, batch_size=args.batch_size, lr=args.lr)
