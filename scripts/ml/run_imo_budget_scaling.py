#!/usr/bin/env python3
"""
Experiment 1: Budget Scaling Ladder on IMO 1960 P2.

Evaluates test/imo_1960_p2 under:
- Condition C0: Single-Tree DBF-MCTS (1 tree, full budget B).
- Condition C2: Persistent Multi-Branch Forest (K=4 persistent branches, interleaved round-robin).

Across a budget scaling ladder:
- B = 60s (1 minute)
- B = 300s (5 minutes)

Measures:
- Solved status (kernel-verified QED)
- Max search depth reached
- Node expansions & simulation count
- Trajectory of the radical hypothesis branch ('have h := Real.sq_sqrt h₀')
- Resource accounting (Lean execution time vs model inference time)
"""

import sys
import json
import time
import argparse
from pathlib import Path
from typing import Dict, Any, List, Optional

ROOT_DIR = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT_DIR))
sys.path.insert(0, str(ROOT_DIR / "scripts" / "ml"))

from dbf_mcts_prover import DBFMCTSProver
from persistent_multi_branch_forest import MultiBranchSearchForest, ForestSearchResult
from lean_repl_client import LeanREPLClient


def run_imo_scaling(budgets: List[float] = [60.0, 300.0], conditions: List[str] = ["C0", "C2"], out_file: Optional[Path] = None):
    print("==================================================================")
    print("EXPERIMENT 1: BUDGET SCALING LADDER ON IMO 1960 P2")
    print(f"Tested Budgets: {budgets} seconds | Conditions: {conditions}")
    print("==================================================================")

    data_file = ROOT_DIR / "data" / "minif2f_full_488.jsonl"
    target_problem = None
    with open(data_file) as f:
        for line in f:
            d = json.loads(line)
            if d.get("problem_id") == "test/imo_1960_p2":
                target_problem = d
                break

    assert target_problem is not None, "test/imo_1960_p2 not found in dataset"

    prover = DBFMCTSProver(use_repl=True)
    forest = MultiBranchSearchForest(prover=prover, k_branches=4, step_chunk_delta=4)

    ladder_results = []

    for b_sec in budgets:
        print(f"\n==================================================================")
        print(f"EVALUATING AT BUDGET B = {b_sec:.1f}s ({b_sec / 60.0:.1f} min)")
        print(f"==================================================================")

        for cond in conditions:
            print(f"\n--- Running Condition {cond} (Budget: {b_sec:.1f}s) ---")
            t0 = time.perf_counter()
            res = forest.search(
                problem=target_problem,
                condition=cond,
                aggregate_budget_sec=b_sec,
            )
            elapsed = time.perf_counter() - t0

            print(f"  Result: Solved={res.solved} in {res.total_aggregate_time_ms:.1f}ms")
            print(f"  Nodes Expanded: {res.total_node_expansions} | Sims: {res.total_simulations}")
            print(f"  Lean Time: {res.total_lean_time_ms:.1f}ms | Inference Time: {res.total_inference_time_ms:.1f}ms")
            print(f"  Termination: {res.termination_reason}")

            # Inspect branches
            for b_info in res.branch_summaries:
                print(f"    Branch {b_info['branch_id']}: root='{b_info['root_tactic']}' | visits={b_info['visits']} | depth={b_info['max_depth_reached']} | q={b_info['q_value']:.3f}")

            ladder_results.append({
                "budget_sec": b_sec,
                "condition": cond,
                "solved": res.solved,
                "total_aggregate_time_ms": res.total_aggregate_time_ms,
                "total_lean_time_ms": res.total_lean_time_ms,
                "total_inference_time_ms": res.total_inference_time_ms,
                "total_node_expansions": res.total_node_expansions,
                "total_simulations": res.total_simulations,
                "termination_reason": res.termination_reason,
                "branch_summaries": res.branch_summaries,
            })

    prover.close()

    # Summary table
    print("\n==================================================================")
    print("BUDGET SCALING LADDER SUMMARY ON IMO 1960 P2")
    print("==================================================================")
    print(f"{'Budget (s)':<12} | {'Condition':<10} | {'Solved':<8} | {'Nodes':<8} | {'Sims':<8} | {'Max Depth':<10} | {'Lean Time':<12}")
    print("-" * 75)
    for r in ladder_results:
        max_d = max([b["max_depth_reached"] for b in r["branch_summaries"]], default=0)
        print(f"{r['budget_sec']:<12.1f} | {r['condition']:<10} | {str(r['solved']):<8} | {r['total_node_expansions']:<8} | {r['total_simulations']:<8} | {max_d:<10} | {r['total_lean_time_ms']:>8.1f} ms")

    if out_file is None:
        out_file = ROOT_DIR / "data" / "clean_minif2f_eval" / "imo_budget_scaling_results.json"
    existing_results = []
    if out_file.exists():
        try:
            with open(out_file) as f:
                existing_results = json.load(f)
        except Exception:
            existing_results = []
    merged_results = existing_results + ladder_results
    with open(out_file, "w") as f:
        json.dump(merged_results, f, indent=2)
    print(f"\nResults saved to: {out_file}")
    print("==================================================================")
    return ladder_results


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Experiment 1: IMO Budget Scaling Ladder")
    parser.add_argument("--budgets", type=str, default="60,300", help="Comma-separated budget seconds")
    parser.add_argument("--conditions", type=str, default="C0,C2", help="Comma-separated conditions (C0, C1, C2, C4)")
    args = parser.parse_args()

    budget_list = [float(b.strip()) for b in args.budgets.split(",")]
    cond_list = [c.strip() for c in args.conditions.split(",")]
    run_imo_scaling(budgets=budget_list, conditions=cond_list)
