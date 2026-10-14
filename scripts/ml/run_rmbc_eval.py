#!/usr/bin/env python3
"""
Evaluation Runner for Recursive Milestone Beam Commit (RMBC).

Evaluates RMBC on:
1. IMO 1960 P2 (`test/imo_1960_p2`).
2. miniF2F `valid` 10-problem balanced benchmark slice.

Measures:
- Solved status (kernel-verified with #print axioms, 0 sorryAx)
- Milestone commits (subgoals closed, lemmas proved)
- Number of competing trees destroyed upon milestone approval
- Resource accounting (Lean kernel time vs ProofBDH inference time)
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
from rmbc_controller import RecursiveMilestoneBeamCommitProver, RMBCSearchResult


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


def run_rmbc_on_problem(
    problem: Dict[str, Any],
    budget_sec: float = 60.0,
    min_score: float = 0.15,
    max_score: float = 1.0,
    beam_width: int = 15,
) -> RMBCSearchResult:
    prover = DBFMCTSProver(use_repl=True)
    rmbc = RecursiveMilestoneBeamCommitProver(
        prover=prover,
        min_score=min_score,
        max_score=max_score,
        beam_width=beam_width,
    )
    res = rmbc.search(problem, aggregate_budget_sec=budget_sec)
    prover.close()
    return res


def run_rmbc_imo(budget_sec: float = 60.0, out_file: Optional[Path] = None):
    print("==================================================================")
    print("EVALUATING RMBC CONTROLLER ON IMO 1960 P2")
    print(f"Aggregate Compute Budget: {budget_sec:.1f}s")
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

    res = run_rmbc_on_problem(target_problem, budget_sec=budget_sec)

    print(f"\nRMBC Result on IMO 1960 P2:")
    print(f"  Solved: {res.solved} in {res.total_aggregate_time_ms:.1f}ms")
    print(f"  Lean Kernel Time: {res.total_lean_time_ms:.1f}ms | Neural Inf Time: {res.total_inference_time_ms:.1f}ms")
    print(f"  Expansions: {res.total_node_expansions} | Sims: {res.total_simulations}")
    print(f"  Competing Trees Destroyed: {res.trees_destroyed_count}")
    print(f"  Termination Reason: {res.termination_reason}")
    print(f"  Milestones Committed ({len(res.milestone_commits)}):")
    for m in res.milestone_commits:
        print(f"    Step {m['step_index']}: [{m['kind']}] '{m['tactic']}' (prior: {m['prior']:.3f}, latency: {m['latency_ms']:.1f}ms)")
    if res.winning_script:
        print(f"  Winning Script:\n{res.winning_script}")

    out_data = {
        "problem_id": res.problem_id,
        "solved": res.solved,
        "winning_script": res.winning_script,
        "total_aggregate_time_ms": res.total_aggregate_time_ms,
        "total_lean_time_ms": res.total_lean_time_ms,
        "total_inference_time_ms": res.total_inference_time_ms,
        "total_node_expansions": res.total_node_expansions,
        "total_simulations": res.total_simulations,
        "trees_destroyed_count": res.trees_destroyed_count,
        "milestone_commits": res.milestone_commits,
        "termination_reason": res.termination_reason,
    }

    if out_file is None:
        out_file = ROOT_DIR / "data" / "clean_minif2f_eval" / "rmbc_imo_results.json"
    out_file.parent.mkdir(parents=True, exist_ok=True)
    with open(out_file, "w") as f:
        json.dump(out_data, f, indent=2)
    print(f"\nTelemetry saved to: {out_file}")
    print("==================================================================")
    return res


def run_rmbc_valid_slice(budget_sec: float = 15.0, num_theorems: int = 10, out_file: Optional[Path] = None):
    print("==================================================================")
    print("EVALUATING RMBC CONTROLLER ON MINIF2F VALID SLICE")
    print(f"Budget per theorem: {budget_sec:.1f}s | Num Theorems: {num_theorems}")
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
    rmbc = RecursiveMilestoneBeamCommitProver(prover=prover, min_score=0.30, max_score=0.90, beam_width=5)

    results: List[RMBCSearchResult] = []

    for idx, prob in enumerate(target_problems, 1):
        pid = prob["problem_id"]
        print(f"[{idx}/{len(target_problems)}] Evaluating: {pid} ... ", end="", flush=True)
        res = rmbc.search(prob, aggregate_budget_sec=budget_sec)
        results.append(res)
        status_str = "SOLVED" if res.solved else "UNSOLVED"
        print(f"{status_str} in {res.total_aggregate_time_ms:.1f}ms (expansions: {res.total_node_expansions}, destroyed: {res.trees_destroyed_count})")

    prover.close()

    n_solved = sum(1 for r in results if r.solved)
    pass_rate = (n_solved / len(results)) * 100.0 if results else 0.0
    avg_time = sum(r.total_aggregate_time_ms for r in results) / len(results) if results else 0.0
    avg_exp = sum(r.total_node_expansions for r in results) / len(results) if results else 0.0
    total_trees_destroyed = sum(r.trees_destroyed_count for r in results)

    print("\n==================================================================")
    print("RMBC VALIDATION BENCHMARK SUMMARY")
    print("==================================================================")
    print(f"Solved: {n_solved}/{len(results)} ({pass_rate:.1f}%)")
    print(f"Avg Expansions: {avg_exp:.1f} | Avg Latency: {avg_time:.1f} ms")
    print(f"Total Sibling Trees Destroyed Upon Milestone Approval: {total_trees_destroyed}")
    print("==================================================================")

    if out_file is None:
        out_file = ROOT_DIR / "data" / "clean_minif2f_eval" / "rmbc_valid_slice_results.json"
    out_file.parent.mkdir(parents=True, exist_ok=True)
    with open(out_file, "w") as f:
        json.dump({
            "num_theorems": len(results),
            "num_solved": n_solved,
            "pass_rate_pct": pass_rate,
            "avg_latency_ms": avg_time,
            "avg_expansions": avg_exp,
            "total_trees_destroyed": total_trees_destroyed,
            "results": [
                {
                    "problem_id": r.problem_id,
                    "solved": r.solved,
                    "winning_script": r.winning_script,
                    "latency_ms": r.total_aggregate_time_ms,
                    "expansions": r.total_node_expansions,
                    "trees_destroyed": r.trees_destroyed_count,
                    "milestones": r.milestone_commits,
                    "termination_reason": r.termination_reason,
                }
                for r in results
            ]
        }, f, indent=2)
    print(f"Results saved to: {out_file}")
    return results


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="RMBC Evaluation Runner")
    parser.add_argument("--target", type=str, default="imo_1960_p2", choices=["imo_1960_p2", "valid_slice"])
    parser.add_argument("--budget", type=float, default=60.0)
    parser.add_argument("--theorems", type=int, default=10)
    args = parser.parse_args()

    if args.target == "imo_1960_p2":
        run_rmbc_imo(budget_sec=args.budget)
    else:
        run_rmbc_valid_slice(budget_sec=args.budget, num_theorems=args.theorems)
