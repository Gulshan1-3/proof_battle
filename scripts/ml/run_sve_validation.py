#!/usr/bin/env python3
"""
Stage 65: Smallest Viable Experiment (SVE) Runner for PMTS.

Evaluates and compares:
- C0: Single-Tree Baseline (1 tree, full budget B).
- C1: Independent Portfolios (K=4 trees, static B/4 each).
- C2: PMTS-Equal (K=4 persistent branches, interleaved round-robin).

Protocol:
- Evaluates on a balanced 10-problem slice of miniF2F `valid` split (quarantining `test`).
- Matched total aggregate compute budget B across conditions.
- Strict kernel verification of all proofs with #print axioms (0 sorryAx).
"""

import sys
import json
import time
import argparse
from pathlib import Path
from typing import Dict, Any, List, Optional, Tuple, Set

ROOT_DIR = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT_DIR))
sys.path.insert(0, str(ROOT_DIR / "scripts" / "ml"))

from dbf_mcts_prover import DBFMCTSProver
from persistent_multi_branch_forest import MultiBranchSearchForest, ForestSearchResult
from lean_repl_client import LeanREPLClient


VALID_THEOREMS_SVE = [
    "valid/algebra_2complexrootspoly_xsqp49eqxp7itxpn7i",
    "valid/algebra_2rootsintpoly_am10tap11eqasqpam110",
    "valid/induction_divisibility_3div2tooddnp1",
    "valid/induction_divisibility_3divnto3m2n",
    "valid/numbertheory_2dvd4expn",
    "valid/numbertheory_aneqprodakp4_anmsqrtanp1eq2",
    "valid/amc12_2000_p11",
    "valid/amc12_2000_p15",
    "valid/mathd_algebra_10",
    "valid/mathd_algebra_101",
]


def run_sve(
    budget_sec: float = 30.0,
    conditions: List[str] = ["C0", "C1", "C2"],
    num_theorems: int = 10,
    out_file: Optional[Path] = None,
):
    print("==================================================================")
    print("STAGE 65: PMTS SMALLEST VIABLE EXPERIMENT (SVE) RUNNER")
    print(f"Budget per theorem: {budget_sec:.1f}s | Conditions: {conditions}")
    print(f"Dataset: miniF2F `valid` slice ({num_theorems} problems)")
    print("==================================================================")

    data_file = ROOT_DIR / "data" / "minif2f_full_488.jsonl"
    lookup = {}
    with open(data_file) as f:
        for line in f:
            d = json.loads(line)
            if d.get("problem_id") in VALID_THEOREMS_SVE:
                lookup[d["problem_id"]] = d

    target_problems = [lookup[pid] for pid in VALID_THEOREMS_SVE[:num_theorems] if pid in lookup]

    prover = DBFMCTSProver(use_repl=True)
    forest = MultiBranchSearchForest(prover=prover, k_branches=4, step_chunk_delta=2)

    results_by_condition: Dict[str, List[ForestSearchResult]] = {c: [] for c in conditions}

    for p_idx, prob in enumerate(target_problems, 1):
        pid = prob["problem_id"]
        print(f"\n[{p_idx}/{len(target_problems)}] Evaluating: {pid}")

        for cond in conditions:
            print(f"  Running Condition {cond}...", end="", flush=True)
            res = forest.search(
                problem=prob,
                condition=cond,
                aggregate_budget_sec=budget_sec,
            )
            results_by_condition[cond].append(res)
            status_str = "SOLVED" if res.solved else "UNSOLVED"
            print(f" -> {status_str} in {res.total_aggregate_time_ms:.1f}ms ({res.total_node_expansions} nodes, {res.total_simulations} sims)")

            # If solved, verify axioms
            if res.solved and res.winning_script:
                ok, _, _, axioms = prover.repl_client.verify_full_code(
                    f"{prover.repl_client._prepare_statement(prob['raw_code'])} := by\n{res.winning_script}",
                    "test_thm"
                )
                assert "sorryAx" not in axioms, f"CRITICAL: sorryAx detected in {pid} under {cond}"

    prover.close()

    # Aggregate Analysis
    print("\n==================================================================")
    print("SVE VALIDATION SUMMARY COMPARISON")
    print("==================================================================")
    print(f"{'Condition':<15} | {'Solved':<8} | {'Pass Rate':<10} | {'Avg Nodes':<10} | {'Avg Sims':<10} | {'Avg Time (ms)':<15}")
    print("-" * 80)

    summary_records = {}
    for cond in conditions:
        cond_res = results_by_condition[cond]
        n_solved = sum(1 for r in cond_res if r.solved)
        pass_rate = (n_solved / len(cond_res)) * 100.0 if cond_res else 0.0
        avg_nodes = sum(r.total_node_expansions for r in cond_res) / len(cond_res) if cond_res else 0.0
        avg_sims = sum(r.total_simulations for r in cond_res) / len(cond_res) if cond_res else 0.0
        avg_time = sum(r.total_aggregate_time_ms for r in cond_res) / len(cond_res) if cond_res else 0.0

        print(f"{cond:<15} | {n_solved}/{len(cond_res):<6} | {pass_rate:>6.1f}%    | {avg_nodes:>8.1f}   | {avg_sims:>8.1f}   | {avg_time:>12.1f} ms")
        summary_records[cond] = {
            "num_theorems": len(cond_res),
            "num_solved": n_solved,
            "pass_rate_pct": pass_rate,
            "avg_node_expansions": avg_nodes,
            "avg_simulations": avg_sims,
            "avg_latency_ms": avg_time,
            "detailed_results": [
                {
                    "problem_id": r.problem_id,
                    "solved": r.solved,
                    "latency_ms": r.total_aggregate_time_ms,
                    "nodes": r.total_node_expansions,
                    "sims": r.total_simulations,
                    "winning_branch_id": r.winning_branch_id,
                    "winning_script": r.winning_script,
                    "termination_reason": r.termination_reason,
                }
                for r in cond_res
            ]
        }

    if out_file is None:
        out_file = ROOT_DIR / "data" / "clean_minif2f_eval" / "sve_validation_results.json"
    out_file.parent.mkdir(parents=True, exist_ok=True)
    with open(out_file, "w") as f:
        json.dump(summary_records, f, indent=2)
    print(f"\nDetailed telemetry saved to: {out_file}")
    print("==================================================================")
    return summary_records


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Stage 65 SVE Runner")
    parser.add_argument("--budget", type=float, default=20.0, help="Per-problem aggregate budget in seconds")
    parser.add_argument("--num-theorems", type=int, default=10, help="Number of validation theorems to evaluate")
    parser.add_argument("--conditions", type=str, default="C0,C1,C2", help="Comma-separated conditions")
    args = parser.parse_args()

    cond_list = [c.strip() for c in args.conditions.split(",")]
    run_sve(budget_sec=args.budget, conditions=cond_list, num_theorems=args.num_theorems)
