#!/usr/bin/env python3
"""
Stage 65: Persistent Multi-Branch Search Forest (PMTS) for Neural Theorem Proving.

Core Research Architecture:
1. Search Portfolio Initialization: Preserves K distinct root branches from top-ranked policy actions.
2. Persistent Branch State: Tracks visit counts, values, resource consumption (Lean execution time,
   model inference time, model calls, node expansions), action trajectories, and disk checkpoints.
3. Strict Aggregate Compute Accounting: Enforces sum_k (lean_time_k + inference_time_k) <= B_max.
4. AND/OR Proof-Tree Semantics: Subgoals are AND-nodes (all subgoals must be discharged to solve);
   alternative branches are OR-nodes. Solving one subgoal awards partial progress but NOT QED.
5. Schedulers:
   - C0: Single-tree baseline (K=1, full budget in one tree).
   - C1: Independent static portfolios (K=4, B/4 per branch in isolation).
   - C2: PMTS-Equal (K=4, round-robin interleaved step allocation in increments of delta sims).
   - C3: PMTS-Prior (K=4, step allocation weighted by root policy prior).
"""

import os
import sys
import math
import time
import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Dict, Any, List, Optional, Tuple, Set

ROOT_DIR = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT_DIR))
sys.path.insert(0, str(ROOT_DIR / "scripts" / "ml"))

from dbf_mcts_prover import (
    DBFMCTSProver,
    DBFMCTSNode,
    DBFMCTSResult,
    parse_subgoal_hypotheses,
    extract_introduced_hypotheses,
    categorize_lean_error,
)
from lean_repl_client import LeanREPLClient

try:
    from learned_allocation_controller import LearnedAllocationController
except ImportError:
    LearnedAllocationController = None


@dataclass
class BranchState:
    """Represents an active, persistent search branch rooted at an initial candidate tactic."""
    branch_id: int
    root_tactic: str
    prior: float
    root_node: DBFMCTSNode
    visits: int = 0
    total_value: float = 0.0
    lean_tactic_time_ms: float = 0.0
    model_inference_time_ms: float = 0.0
    model_call_count: int = 0
    node_expansions: int = 0
    is_solved: bool = False
    is_exhausted: bool = False
    winning_script: Optional[str] = None
    action_trajectory: List[str] = field(default_factory=list)
    remaining_subgoals: List[str] = field(default_factory=list)
    max_depth_reached: int = 0

    @property
    def q_value(self) -> float:
        return self.total_value / self.visits if self.visits > 0 else 0.0

    @property
    def total_cost_ms(self) -> float:
        return self.lean_tactic_time_ms + self.model_inference_time_ms

    def to_dict(self) -> Dict[str, Any]:
        return {
            "branch_id": self.branch_id,
            "root_tactic": self.root_tactic,
            "prior": self.prior,
            "visits": self.visits,
            "total_value": self.total_value,
            "q_value": self.q_value,
            "lean_tactic_time_ms": self.lean_tactic_time_ms,
            "model_inference_time_ms": self.model_inference_time_ms,
            "total_cost_ms": self.total_cost_ms,
            "model_call_count": self.model_call_count,
            "node_expansions": self.node_expansions,
            "is_solved": self.is_solved,
            "is_exhausted": self.is_exhausted,
            "winning_script": self.winning_script,
            "action_trajectory": self.action_trajectory,
            "remaining_subgoals": self.remaining_subgoals,
            "max_depth_reached": self.max_depth_reached,
        }

    def save_checkpoint(self, filepath: Path):
        """Serializes branch state to a JSON checkpoint on disk."""
        filepath.parent.mkdir(parents=True, exist_ok=True)
        with open(filepath, "w") as f:
            json.dump(self.to_dict(), f, indent=2)

    @classmethod
    def load_checkpoint(cls, filepath: Path, prover: DBFMCTSProver, raw_code: str) -> "BranchState":
        """Restores a branch from disk and deterministically reconstructs proof state."""
        with open(filepath) as f:
            d = json.load(f)

        # Reconstruct root node
        root = DBFMCTSNode(
            node_id=f"branch_{d['branch_id']}_root",
            depth=1,
            goal="",
            tactic=d["root_tactic"],
            accumulated_script=d["root_tactic"],
            prior=d["prior"],
            visits=d["visits"],
            total_value=d["total_value"],
        )

        branch = cls(
            branch_id=d["branch_id"],
            root_tactic=d["root_tactic"],
            prior=d["prior"],
            root_node=root,
            visits=d["visits"],
            total_value=d["total_value"],
            lean_tactic_time_ms=d["lean_tactic_time_ms"],
            model_inference_time_ms=d["model_inference_time_ms"],
            model_call_count=d["model_call_count"],
            node_expansions=d["node_expansions"],
            is_solved=d["is_solved"],
            is_exhausted=d["is_exhausted"],
            winning_script=d["winning_script"],
            action_trajectory=d["action_trajectory"],
            remaining_subgoals=d["remaining_subgoals"],
            max_depth_reached=d["max_depth_reached"],
        )
        return branch


@dataclass
class ForestSearchResult:
    """Outcome of multi-branch search across the portfolio."""
    problem_id: str
    solved: bool
    winning_script: Optional[str]
    winning_branch_id: Optional[int]
    condition: str
    total_aggregate_time_ms: float
    total_lean_time_ms: float
    total_inference_time_ms: float
    total_node_expansions: int
    total_simulations: int
    unique_states_count: int
    duplicate_state_count: int
    branch_summaries: List[Dict[str, Any]]
    termination_reason: str


class MultiBranchSearchForest:
    """
    Search controller for Persistent Multi-Branch Search Forests (PMTS).
    Manages K persistent branches rooted in top candidate tactics, allocates
    simulation budgets under an aggregate compute cap, and verifies proofs.
    """

    def __init__(
        self,
        prover: DBFMCTSProver,
        k_branches: int = 4,
        step_chunk_delta: int = 4,
        max_branch_depth: int = 8,
        checkpoint_dir: Optional[Path] = None,
    ):
        self.prover = prover
        self.k_branches = k_branches
        self.step_chunk_delta = step_chunk_delta
        self.max_branch_depth = max_branch_depth
        self.checkpoint_dir = checkpoint_dir

    def search(
        self,
        problem: Dict[str, Any],
        condition: str = "C2",  # C0: single-tree, C1: independent, C2: pmts-equal, C3: pmts-prior
        aggregate_budget_sec: float = 45.0,
        random_seed: int = 42,
    ) -> ForestSearchResult:
        """
        Executes multi-branch search under condition C0, C1, C2, or C3
        with strict aggregate compute accounting.
        """
        t_global_start = time.perf_counter()
        pid = problem.get("problem_id", "unknown")
        goal = problem.get("goal", "")
        params = problem.get("params", "")
        raw_code = problem.get("raw_code", "")

        budget_ms = aggregate_budget_sec * 1000.0

        # Step 1: Initialize Root State in Lean REPL
        root_ps_id = None
        initial_goals = []
        if self.prover.repl_client is not None:
            try:
                root_ps_id, initial_goals, _ = self.prover.repl_client.init_proof_state(raw_code)
            except Exception:
                root_ps_id = None
                initial_goals = []

        # Step 2: Query Policy for Root Candidate Actions
        t_inf_0 = time.perf_counter()
        num_candidates = 1 if condition == "C0" else max(self.k_branches, 4)
        root_candidates = self.prover.generate_candidate_actions(goal, params, num_candidates=num_candidates)
        initial_inf_time = (time.perf_counter() - t_inf_0) * 1000.0

        if condition == "C0":
            # Single-tree uses top-1 candidate or standard root
            candidates_to_use = root_candidates[:1]
        else:
            candidates_to_use = root_candidates[:self.k_branches]

        # Step 3: Initialize Branches
        branches: List[BranchState] = []
        visited_states: Set[str] = set()
        duplicate_states = 0

        for b_idx, (tac, prior) in enumerate(candidates_to_use):
            root_node = DBFMCTSNode(
                node_id=f"b{b_idx}_root",
                depth=1,
                goal=goal,
                tactic=tac,
                accumulated_script=tac,
                accumulated_hypotheses=params,
                prior=prior,
                proof_state_id=root_ps_id,
                remaining_subgoals=initial_goals
            )

            branch = BranchState(
                branch_id=b_idx,
                root_tactic=tac,
                prior=prior,
                root_node=root_node,
                action_trajectory=[tac],
                model_inference_time_ms=initial_inf_time / len(candidates_to_use),
                model_call_count=1,
            )

            # Evaluate initial step
            t_eval_0 = time.perf_counter()
            val = self.prover.evaluate(root_node, raw_code)
            t_eval = (time.perf_counter() - t_eval_0) * 1000.0

            branch.lean_tactic_time_ms += t_eval
            branch.visits += 1
            branch.total_value += val
            branch.node_expansions += 1
            branch.remaining_subgoals = root_node.remaining_subgoals

            branches.append(branch)

            if root_node.is_solved:
                branch.is_solved = True
                branch.winning_script = tac
                break

        if not branches:
            return ForestSearchResult(
                problem_id=pid,
                solved=False,
                winning_script=None,
                winning_branch_id=None,
                condition=condition,
                total_aggregate_time_ms=(time.perf_counter() - t_global_start) * 1000.0,
                total_lean_time_ms=0.0,
                total_inference_time_ms=initial_inf_time,
                total_node_expansions=0,
                total_simulations=0,
                unique_states_count=0,
                duplicate_state_count=0,
                branch_summaries=[],
                termination_reason="no_candidates_generated"
            )

        # Check immediate solution
        solved_branch = next((b for b in branches if b.is_solved), None)
        if solved_branch is not None:
            total_elapsed = (time.perf_counter() - t_global_start) * 1000.0
            return ForestSearchResult(
                problem_id=pid,
                solved=True,
                winning_script=solved_branch.winning_script,
                winning_branch_id=solved_branch.branch_id,
                condition=condition,
                total_aggregate_time_ms=total_elapsed,
                total_lean_time_ms=sum(b.lean_tactic_time_ms for b in branches),
                total_inference_time_ms=sum(b.model_inference_time_ms for b in branches),
                total_node_expansions=sum(b.node_expansions for b in branches),
                total_simulations=sum(b.visits for b in branches),
                unique_states_count=len(visited_states),
                duplicate_state_count=duplicate_states,
                branch_summaries=[b.to_dict() for b in branches],
                termination_reason="solved_at_root"
            )

        # Step 4: Multi-Branch Search Allocation Loop
        termination_reason = "budget_exhausted"

        if condition == "C0":
            # Condition 0: Single-Tree Baseline (All budget allocated to Branch 0)
            self._search_branch_budget(branches[0], raw_code, budget_ms, t_global_start)
            if branches[0].is_solved:
                termination_reason = "solved"

        elif condition == "C1":
            # Condition 1: Independent Portfolios (Static B / K budget per branch)
            per_branch_budget = budget_ms / len(branches)
            for branch in branches:
                if self._get_aggregate_time_ms(branches) >= budget_ms:
                    break
                self._search_branch_budget(branch, raw_code, per_branch_budget, t_global_start, global_cap_ms=budget_ms, all_branches=branches)
                if branch.is_solved:
                    termination_reason = "solved"
                    break

        elif condition == "C2":
            # Condition 2: PMTS-Equal (Interleaved Round-Robin in increments of delta simulations)
            while self._get_aggregate_time_ms(branches) < budget_ms:
                active_branches = [b for b in branches if not b.is_exhausted and not b.is_solved]
                if not active_branches:
                    termination_reason = "all_branches_exhausted"
                    break

                progress_made = False
                for branch in active_branches:
                    if self._get_aggregate_time_ms(branches) >= budget_ms:
                        break
                    sims_before = branch.visits
                    self._step_branch_simulations(branch, raw_code, num_sims=self.step_chunk_delta, budget_ms=budget_ms, all_branches=branches)
                    if branch.visits > sims_before:
                        progress_made = True

                    if branch.is_solved:
                        termination_reason = "solved"
                        break

                if any(b.is_solved for b in branches):
                    break
                if not progress_made:
                    termination_reason = "all_branches_stalled"
                    break

        elif condition == "C3":
            # Condition 3: PMTS-Prior Weighted Allocation
            priors = [b.prior for b in branches]
            prior_sum = sum(priors) if sum(priors) > 0 else 1.0
            weights = [p / prior_sum for p in priors]

            while self._get_aggregate_time_ms(branches) < budget_ms:
                active_branches = [b for b in branches if not b.is_exhausted and not b.is_solved]
                if not active_branches:
                    termination_reason = "all_branches_exhausted"
                    break

                for branch, weight in zip(branches, weights):
                    if branch.is_exhausted or branch.is_solved:
                        continue
                    if self._get_aggregate_time_ms(branches) >= budget_ms:
                        break
                    allocated_sims = max(2, int(round(self.step_chunk_delta * weight * len(branches))))
                    self._step_branch_simulations(branch, raw_code, num_sims=allocated_sims, budget_ms=budget_ms, all_branches=branches)
                    if branch.is_solved:
                        termination_reason = "solved"
                        break

                if any(b.is_solved for b in branches):
                    break

        elif condition == "C4":
            # Condition 4: LAMP - Learned Adaptive Monte Carlo Proof Search Controller
            controller = LearnedAllocationController() if LearnedAllocationController is not None else None
            while self._get_aggregate_time_ms(branches) < budget_ms:
                active_branches = [b for b in branches if not b.is_exhausted and not b.is_solved]
                if not active_branches:
                    termination_reason = "all_branches_exhausted"
                    break

                if controller is not None:
                    best_idx, alloc_delta = controller.select_next_branch(
                        branches,
                        global_budget_ms=budget_ms,
                        step_chunk_delta=self.step_chunk_delta
                    )
                else:
                    best_idx = 0
                    alloc_delta = self.step_chunk_delta

                branch_to_step = branches[best_idx]
                sims_before = branch_to_step.visits
                self._step_branch_simulations(
                    branch_to_step,
                    raw_code,
                    num_sims=alloc_delta,
                    budget_ms=budget_ms,
                    all_branches=branches
                )

                if branch_to_step.is_solved:
                    termination_reason = "solved"
                    break

                if branch_to_step.visits == sims_before and branch_to_step.is_exhausted:
                    continue

        total_elapsed = (time.perf_counter() - t_global_start) * 1000.0
        winning_branch = next((b for b in branches if b.is_solved), None)

        return ForestSearchResult(
            problem_id=pid,
            solved=(winning_branch is not None),
            winning_script=winning_branch.winning_script if winning_branch else None,
            winning_branch_id=winning_branch.branch_id if winning_branch else None,
            condition=condition,
            total_aggregate_time_ms=total_elapsed,
            total_lean_time_ms=sum(b.lean_tactic_time_ms for b in branches),
            total_inference_time_ms=sum(b.model_inference_time_ms for b in branches),
            total_node_expansions=sum(b.node_expansions for b in branches),
            total_simulations=sum(b.visits for b in branches),
            unique_states_count=len(visited_states),
            duplicate_state_count=duplicate_states,
            branch_summaries=[b.to_dict() for b in branches],
            termination_reason=termination_reason
        )

    def _get_aggregate_time_ms(self, branches: List[BranchState]) -> float:
        """Returns total consumed compute (Lean tactic time + model inference time)."""
        return sum(b.total_cost_ms for b in branches)

    def _search_branch_budget(
        self,
        branch: BranchState,
        raw_code: str,
        branch_budget_ms: float,
        t_global_start: float,
        global_cap_ms: Optional[float] = None,
        all_branches: Optional[List[BranchState]] = None
    ):
        """Runs simulations on a single branch until its budget allocation is met."""
        t_start = time.perf_counter()
        branch_initial_cost = branch.total_cost_ms

        while (branch.total_cost_ms - branch_initial_cost) < branch_budget_ms:
            if global_cap_ms is not None and all_branches is not None:
                if self._get_aggregate_time_ms(all_branches) >= global_cap_ms:
                    break
            if branch.is_solved or branch.is_exhausted:
                break
            self._execute_one_simulation(branch, raw_code)

    def _step_branch_simulations(
        self,
        branch: BranchState,
        raw_code: str,
        num_sims: int,
        budget_ms: float,
        all_branches: List[BranchState]
    ):
        """Executes up to num_sims iterations on the specified branch."""
        for _ in range(num_sims):
            if self._get_aggregate_time_ms(all_branches) >= budget_ms:
                break
            if branch.is_solved or branch.is_exhausted:
                break
            self._execute_one_simulation(branch, raw_code)

    def _execute_one_simulation(self, branch: BranchState, raw_code: str):
        """Executes a single MCTS simulation step on a branch."""
        root = branch.root_node
        leaf = self.prover.select(root)
        branch.max_depth_reached = max(branch.max_depth_reached, leaf.depth)

        if leaf.is_solved:
            branch.is_solved = True
            branch.winning_script = leaf.accumulated_script
            return

        if leaf.depth >= self.max_branch_depth:
            leaf.is_terminal = True
            return

        # Candidate Expansion
        if not leaf.children and not leaf.is_terminal:
            next_goal = leaf.remaining_subgoals[0] if leaf.remaining_subgoals else leaf.goal
            curr_hyps = leaf.accumulated_hypotheses

            t_inf_0 = time.perf_counter()
            candidates = self.prover.generate_candidate_actions(next_goal, curr_hyps, num_candidates=3)
            inf_time = (time.perf_counter() - t_inf_0) * 1000.0
            branch.model_inference_time_ms += inf_time
            branch.model_call_count += 1

            if not candidates:
                leaf.is_terminal = True
                return

            for c_idx, (tac, prior) in enumerate(candidates):
                new_script = f"{leaf.accumulated_script}\n{tac}" if leaf.accumulated_script else tac
                child = DBFMCTSNode(
                    node_id=f"{leaf.node_id}_{c_idx}",
                    depth=leaf.depth + 1,
                    goal=next_goal,
                    tactic=tac,
                    accumulated_script=new_script,
                    accumulated_hypotheses=curr_hyps,
                    parent=leaf,
                    prior=prior,
                )
                leaf.children.append(child)
                branch.node_expansions += 1

                t_eval_0 = time.perf_counter()
                val = self.prover.evaluate(child, raw_code)
                eval_time = (time.perf_counter() - t_eval_0) * 1000.0
                branch.lean_tactic_time_ms += eval_time

                self.prover.backpropagate(child, val)
                branch.visits += 1
                branch.total_value += val

                if child.is_solved:
                    # AND/OR Subgoal Verification:
                    # If this child closed all goals, mark branch solved
                    branch.is_solved = True
                    branch.winning_script = child.accumulated_script
                    return
        else:
            # Reached a terminal or visited leaf without expansions
            leaf.is_terminal = True
            branch.is_exhausted = all(c.is_terminal for c in root.children) if root.children else True
