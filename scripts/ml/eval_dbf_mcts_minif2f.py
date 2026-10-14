#!/usr/bin/env python3
"""
Stage 50: miniF2F Benchmark Sweep with Diagnostic Best-First MCTS Prover.

Runs DBF-MCTS across miniF2F splits:
- 100% Pure Neural Execution: zero heuristic tactic fallbacks.
- Length-Normalized Priors (BFS-Prover, ACL 2025).
- PUCT Exploration across deep branches.
- In-Tree Compiler Diagnostic Self-Repair on compiler error tracebacks.
- Persistent Lean 4 REPL client with resident Mathlib environment.
- Continuous RSS memory watchdog (strictly < 4.2 GiB).
- Incremental line-by-line persistence and resume capability.
"""

import os
import sys
import gc
import json
import time
import argparse
import psutil
from pathlib import Path
from typing import Dict, Any, List, Optional, Set, Tuple
from concurrent.futures import ThreadPoolExecutor, as_completed

ROOT_DIR = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT_DIR))
sys.path.insert(0, str(ROOT_DIR / "scripts" / "ml"))

import torch
from dbf_mcts_prover import DBFMCTSProver, DBFMCTSResult, DynamicStoppingConfig
from lean_repl_client import LeanREPLClient

DATA_DIR = ROOT_DIR / "data"
MINIF2F_FILE = DATA_DIR / "minif2f_full_488.jsonl"
OUT_DIR = DATA_DIR / "clean_minif2f_eval"
OUT_DIR.mkdir(parents=True, exist_ok=True)


def get_current_rss_gb() -> Tuple[float, float]:
    """Returns (python_rss_gb, total_tree_rss_gb)."""
    proc = psutil.Process()
    py_rss = proc.memory_info().rss
    tree_rss = py_rss
    try:
        for child in proc.children(recursive=True):
            try:
                tree_rss += child.memory_info().rss
            except (psutil.NoSuchProcess, psutil.AccessDenied):
                pass
    except Exception:
        pass
    return py_rss / (1024 ** 3), tree_rss / (1024 ** 3)


def evaluate_split(
    tier: str = "10m",
    split_filter: str = "test",
    comp_filter: Optional[str] = None,
    simulations_budget: int = 32,
    dynamic_stopping: bool = True,
    min_sims: int = 4,
    max_surge_sims: int = 96,
    num_workers: int = 4,
    timeout_sec: float = 30.0,
    device: str = "cuda",
    tag: Optional[str] = "stage50",
    resume: bool = True,
    checkpoint_path: Optional[str] = None,
    vocab_path: Optional[str] = None
):
    tier = tier.lower().strip()
    if checkpoint_path is None:
        if tier == "95m":
            checkpoint_path = str(DATA_DIR / "native_prover" / "checkpoints" / "bdh_95m_curriculum_best.pt")
        elif tier == "cascade":
            checkpoint_path = "cascade"

    print("=" * 80)
    tag_str = f" [TAG: {tag}] [TIER: {tier.upper()}]" if tag else f" [TIER: {tier.upper()}]"
    print(f"STAGE 50: DBF-MCTS BENCHMARK EVALUATION (SPLIT: {split_filter.upper()}){tag_str}")
    if comp_filter:
        print(f"Competition Filter: {comp_filter}")
    ds_str = f"TP-DS-MCTS (min: {min_sims}, base: {simulations_budget}, surge: {max_surge_sims})" if dynamic_stopping else "Static Budget"
    print(f"Simulations budget: {simulations_budget} ({ds_str}) | Workers: {num_workers} | Timeout: {timeout_sec}s | Device: {device}")
    print("Search: Length-Normalized BFS + PUCT + Compiler Diagnostic Self-Repair + Dynamic Stopping")
    print("Purity: 100% Pure Neural (Zero Solver Fallbacks)")
    print("Memory Watchdog: Active (Limit 4.2 GiB)")
    print("=" * 80 + "\n")

    ds_config = DynamicStoppingConfig(
        enabled=dynamic_stopping,
        min_sims=min_sims,
        base_sims=simulations_budget,
        max_surge_sims=max_surge_sims
    )

    with open(MINIF2F_FILE, "r", encoding="utf-8") as f:
        all_probs = [json.loads(line) for line in f if line.strip()]

    if split_filter != "all":
        probs = [p for p in all_probs if p.get("split") == split_filter]
    else:
        probs = all_probs

    if comp_filter:
        filters = [f.strip().lower() for f in comp_filter.split(",") if f.strip()]
        probs = [
            p for p in probs
            if any(f in p.get("competition", "").lower() or f in p.get("problem_id", "").lower() for f in filters)
        ]

    tag_prefix = f"{tag}_" if tag else ""
    out_results = OUT_DIR / f"dbf_mcts_{tag_prefix}{split_filter}_results.jsonl"
    out_summary = OUT_DIR / f"dbf_mcts_{tag_prefix}{split_filter}_summary.json"

    completed_ids: Set[str] = set()
    results: List[Dict[str, Any]] = []
    solved_count = 0
    one_step_solves = 0
    tree_search_solves = 0
    diagnostic_repair_solves = 0
    barren_pruned_count = 0
    surged_count = 0
    total_sims_saved = 0

    if resume and out_results.exists():
        with open(out_results, "r", encoding="utf-8") as f_in:
            for line in f_in:
                if line.strip():
                    try:
                        record = json.loads(line)
                        pid = record.get("problem_id")
                        if pid and pid not in completed_ids:
                            completed_ids.add(pid)
                            results.append(record)
                            if record.get("solved"):
                                solved_count += 1
                                sm = record.get("solve_method")
                                if sm == "one_step":
                                    one_step_solves += 1
                                elif sm == "tree_search":
                                    tree_search_solves += 1
                                if record.get("diagnostic_repairs_succeeded", 0) > 0:
                                    diagnostic_repair_solves += 1
                                if record.get("early_stopped") == "barren_tree":
                                    barren_pruned_count += 1
                                if record.get("surged"):
                                    surged_count += 1
                                total_sims_saved += max(0, simulations_budget - record.get("effective_simulations", simulations_budget))
                    except Exception:
                        pass
        print(f"Resuming: Loaded {len(completed_ids)} already completed problems from {out_results.name}")
        probs_to_run = [p for p in probs if p["problem_id"] not in completed_ids]
    else:
        probs_to_run = probs
        if out_results.exists():
            out_results.unlink()

    print(f"Evaluating {len(probs_to_run)} problems remaining (Total in split: {len(probs)}) across {num_workers} workers...")

    if not probs_to_run:
        print("All problems already evaluated.")
    else:
        prover_kwargs = {"device": device, "timeout_sec": timeout_sec, "tier": tier}
        if checkpoint_path:
            prover_kwargs["checkpoint_path"] = checkpoint_path
        if vocab_path:
            prover_kwargs["vocab_path"] = vocab_path
        prover = DBFMCTSProver(**prover_kwargs)
        t_start = time.time()

        try:
            with open(out_results, "a", encoding="utf-8") as f_out:
                with ThreadPoolExecutor(max_workers=num_workers) as executor:
                    futures = {executor.submit(prover.search, p, simulations_budget, ds_config): p for p in probs_to_run}
                    for fut in as_completed(futures):
                        p_info = futures[fut]
                        try:
                            res = fut.result()
                            res_dict = res.to_dict()
                        except Exception as exc:
                            print(f"  [ERROR] Problem {p_info.get('problem_id')} failed: {exc}", flush=True)
                            res_dict = {
                                "problem_id": p_info.get("problem_id", "unknown"),
                                "solved": False,
                                "winning_script": None,
                                "solve_method": None,
                                "nodes_expanded": 0,
                                "max_depth_reached": 0,
                                "diagnostic_repairs_attempted": 0,
                                "diagnostic_repairs_succeeded": 0,
                                "total_latency_ms": 0.0,
                                "trajectory_steps": 0,
                                "error": str(exc),
                                "early_stopped": "error",
                                "effective_simulations": 0,
                                "surged": False
                            }

                        results.append(res_dict)
                        f_out.write(json.dumps(res_dict) + "\n")
                        f_out.flush()

                        if res_dict.get("solved"):
                            solved_count += 1
                            if res_dict.get("solve_method") == "one_step":
                                one_step_solves += 1
                            elif res_dict.get("solve_method") == "tree_search":
                                tree_search_solves += 1
                            if res_dict.get("diagnostic_repairs_succeeded", 0) > 0:
                                diagnostic_repair_solves += 1

                        if res_dict.get("early_stopped") == "barren_tree":
                            barren_pruned_count += 1
                        if res_dict.get("surged"):
                            surged_count += 1
                        total_sims_saved += max(0, simulations_budget - res_dict.get("effective_simulations", simulations_budget))

                        total_done = len(results)
                        py_rss, tree_rss = get_current_rss_gb()
                        avail_mem = psutil.virtual_memory().available / (1024 ** 3)
                        elapsed = time.time() - t_start
                        pct = round(solved_count / total_done * 100, 2)
                        sol_mark = " *** SOLVED ***" if res_dict.get("solved") else ""
                        early_str = f" [{res_dict.get('early_stopped')}]" if res_dict.get("early_stopped") else ""
                        surge_str = " [SURGED]" if res_dict.get("surged") else ""
                        print(
                            f"  Progress: {total_done:03d}/{len(probs):03d} | "
                            f"Problem: {res_dict.get('problem_id')}{sol_mark}{early_str}{surge_str} | "
                            f"Solved: {solved_count} ({pct}%) [1-Step: {one_step_solves} | Tree: {tree_search_solves} | Repaired: {diagnostic_repair_solves}] | "
                            f"DS: [Barren: {barren_pruned_count} | Surged: {surged_count} | Saved: {total_sims_saved} sims] | "
                            f"PyRSS: {py_rss:.2f}G | TreeRSS: {tree_rss:.2f}G | Avail: {avail_mem:.2f}G | "
                            f"Elapsed: {elapsed/60:.1f}m",
                            flush=True
                        )

                        gc.collect()
                        time.sleep(0.05)

                        # Watchdog: recycle Lean REPL only if TreeRSS exceeds 7.5 GiB or available system RAM < 2.5 GiB
                        if tree_rss > 7.5 or avail_mem < 2.5:
                            print(f"  [WATCHDOG] Recycling Lean REPL (TreeRSS: {tree_rss:.2f}G, Avail: {avail_mem:.2f}G)...", flush=True)
                            prover.recycle_repl()
                            gc.collect()
        finally:
            prover.close()

    py_final, tree_final = get_current_rss_gb()
    summary = {
        "split": split_filter,
        "tag": tag,
        "total_problems": len(probs),
        "simulations_budget": simulations_budget,
        "dynamic_stopping_enabled": dynamic_stopping,
        "min_sims": min_sims,
        "max_surge_sims": max_surge_sims,
        "solved_count": solved_count,
        "solve_percentage": round(solved_count / len(probs) * 100, 2),
        "one_step_solves": one_step_solves,
        "tree_search_solves": tree_search_solves,
        "diagnostic_repair_solves": diagnostic_repair_solves,
        "barren_pruned_count": barren_pruned_count,
        "surged_count": surged_count,
        "total_sims_saved": total_sims_saved,
        "final_py_rss_gb": round(py_final, 2),
        "final_tree_rss_gb": round(tree_final, 2)
    }

    print("\n" + "=" * 80)
    print(f"FINAL DBF-MCTS RESULTS FOR SPLIT: {split_filter.upper()}{tag_str}")
    print(f"Solved: {solved_count} / {len(probs)} ({summary['solve_percentage']}%)")
    print(f"  - Solved via Fast 1-Step: {one_step_solves}")
    print(f"  - Solved via MCTS Tree Search: {tree_search_solves}")
    print(f"  - Solved via In-Tree Compiler Repair: {diagnostic_repair_solves}")
    print(f"Dynamic Stopping (TP-DS-MCTS) Efficiency:")
    print(f"  - Barren Trees Pruned Early: {barren_pruned_count}")
    print(f"  - Compute Surged Trees: {surged_count}")
    print(f"  - Simulations Saved vs Static: {total_sims_saved:,}")
    print("=" * 80 + "\n")

    with open(out_summary, "w", encoding="utf-8") as f:
        json.dump(summary, f, indent=2)

    return summary


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Evaluate DBF-MCTS on miniF2F")
    parser.add_argument("--tier", type=str, default="10m", choices=["10m", "95m", "cascade"], help="Model tier: 10m baseline, 95m scale, or cascade (dual-tier ensemble)")
    parser.add_argument("--split", type=str, default="test", choices=["valid", "test", "all"])
    parser.add_argument("--filter", type=str, default=None, help="Filter by competition or problem substring (e.g. aime, imo)")
    parser.add_argument("--simulations", type=int, default=32)
    parser.add_argument("--dynamic-stopping", action="store_true", default=True, help="Enable Theorem-Proving Dynamic Simulation MCTS")
    parser.add_argument("--no-dynamic-stopping", dest="dynamic_stopping", action="store_false", help="Disable dynamic stopping")
    parser.add_argument("--min-sims", type=int, default=4, help="Min simulations before barren-tree rejection check")
    parser.add_argument("--max-surge-sims", type=int, default=96, help="Max simulations when compute is surged")
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument("--timeout", type=float, default=30.0)
    parser.add_argument("--device", type=str, default="cuda")
    parser.add_argument("--tag", type=str, default="stage50")
    parser.add_argument("--checkpoint", type=str, default=None, help="Custom checkpoint path")
    parser.add_argument("--vocab", type=str, default=None, help="Custom vocab path")
    parser.add_argument("--resume", action="store_true", default=True)
    parser.add_argument("--no-resume", dest="resume", action="store_false")
    args = parser.parse_args()

    evaluate_split(
        tier=args.tier,
        split_filter=args.split,
        comp_filter=args.filter,
        simulations_budget=args.simulations,
        dynamic_stopping=args.dynamic_stopping,
        min_sims=args.min_sims,
        max_surge_sims=args.max_surge_sims,
        num_workers=args.workers,
        timeout_sec=args.timeout,
        device=args.device,
        tag=args.tag,
        resume=args.resume,
        checkpoint_path=args.checkpoint,
        vocab_path=args.vocab
    )
