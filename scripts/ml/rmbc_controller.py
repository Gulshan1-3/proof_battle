#!/usr/bin/env python3
"""
Recursive Milestone Beam Commit (RMBC) Search Controller.

Implements competitive multi-tree search with hard commitment:
1. Generates top K candidate tactics filtered by policy confidence band [τ_min, τ_max].
2. Spawns independent exploration trees rooted at these candidates.
3. Upon first verified milestone (subgoal closure or kernel-approved lemma):
   - Commits the winning step to the proof script.
   - IMMEDIATELY DESTROYS all competing sibling trees to avoid compute diffusion.
4. Recursively advances the active proof frontier until full QED.
5. Verifies assembled proof via Lean kernel with 0 sorryAx.
"""

import sys
import time
from enum import Enum
from pathlib import Path
from dataclasses import dataclass, field
from typing import Dict, Any, List, Optional, Tuple, Set

ROOT_DIR = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT_DIR))
sys.path.insert(0, str(ROOT_DIR / "scripts" / "ml"))

from dbf_mcts_prover import DBFMCTSProver, DBFMCTSNode
from lean_repl_client import LeanREPLClient


class MilestoneKind(str, Enum):
    FULL_QED = "full_qed"
    SUBGOAL_CLOSED = "subgoal_closed"
    LEMMA_PROVED = "lemma_proved"
    DECOMPOSITION = "decomposition"
    SIMPLIFICATION = "simplification"


@dataclass
class MilestoneCommit:
    step_index: int
    tactic: str
    kind: MilestoneKind
    subgoals_before: int
    subgoals_after: int
    policy_prior: float
    lean_latency_ms: float
    proof_state_id: int
    active_goal_before: str


@dataclass
class RMBCSearchResult:
    problem_id: str
    solved: bool
    winning_script: Optional[str]
    total_aggregate_time_ms: float
    total_lean_time_ms: float
    total_inference_time_ms: float
    total_node_expansions: int
    total_simulations: int
    trees_destroyed_count: int
    milestone_commits: List[Dict[str, Any]]
    termination_reason: str


class RecursiveMilestoneBeamCommitProver:
    """
    Search controller for Recursive Milestone Beam Commit (RMBC).
    """

    def __init__(
        self,
        prover: DBFMCTSProver,
        min_score: float = 0.40,
        max_score: float = 1.0,
        beam_width: int = 8,
        max_recursion_depth: int = 15,
        local_mcts_quantum: int = 4,
    ):
        self.prover = prover
        self.min_score = min_score
        self.max_score = max_score
        self.beam_width = beam_width
        self.max_recursion_depth = max_recursion_depth
        self.local_mcts_quantum = local_mcts_quantum

    def search(
        self,
        problem: Dict[str, Any],
        aggregate_budget_sec: float = 60.0,
    ) -> RMBCSearchResult:
        """
        Executes RMBC search on the given problem under an aggregate compute budget.
        """
        t_global_start = time.perf_counter()
        pid = problem.get("problem_id", "unknown")
        goal = problem.get("goal", "")
        params = problem.get("params", "")
        raw_code = problem.get("raw_code", "")

        budget_ms = aggregate_budget_sec * 1000.0

        total_lean_time_ms = 0.0
        total_inf_time_ms = 0.0
        total_expansions = 0
        total_simulations = 0
        trees_destroyed = 0
        commits: List[MilestoneCommit] = []

        # Step 1: Initialize Lean REPL Proof State
        repl = self.prover.repl_client
        if repl is None:
            return RMBCSearchResult(
                problem_id=pid,
                solved=False,
                winning_script=None,
                total_aggregate_time_ms=0.0,
                total_lean_time_ms=0.0,
                total_inference_time_ms=0.0,
                total_node_expansions=0,
                total_simulations=0,
                trees_destroyed_count=0,
                milestone_commits=[],
                termination_reason="no_repl_available"
            )

        t_init_0 = time.perf_counter()
        curr_ps_id, curr_subgoals, init_err = repl.init_proof_state(raw_code)
        init_time = (time.perf_counter() - t_init_0) * 1000.0
        total_lean_time_ms += init_time

        if curr_ps_id is None:
            return RMBCSearchResult(
                problem_id=pid,
                solved=False,
                winning_script=None,
                total_aggregate_time_ms=(time.perf_counter() - t_global_start) * 1000.0,
                total_lean_time_ms=total_lean_time_ms,
                total_inference_time_ms=0.0,
                total_node_expansions=0,
                total_simulations=0,
                trees_destroyed_count=0,
                milestone_commits=[],
                termination_reason=f"repl_init_failed: {init_err}"
            )

        # Immediate QED check
        if len(curr_subgoals) == 0:
            return RMBCSearchResult(
                problem_id=pid,
                solved=True,
                winning_script="",
                total_aggregate_time_ms=(time.perf_counter() - t_global_start) * 1000.0,
                total_lean_time_ms=total_lean_time_ms,
                total_inference_time_ms=0.0,
                total_node_expansions=0,
                total_simulations=0,
                trees_destroyed_count=0,
                milestone_commits=[],
                termination_reason="solved_initially"
            )

        committed_script_steps: List[str] = []
        recursion_depth = 0
        termination_reason = "budget_exhausted"

        # Main Recursive Milestone Loop
        while recursion_depth < self.max_recursion_depth:
            elapsed_ms = (time.perf_counter() - t_global_start) * 1000.0
            if elapsed_ms >= budget_ms:
                termination_reason = "budget_exhausted"
                break

            if len(curr_subgoals) == 0:
                termination_reason = "solved"
                break

            recursion_depth += 1
            active_subgoal = curr_subgoals[0]
            subgoals_before_count = len(curr_subgoals)

            # 1. Candidate Generation & Filtering by Confidence Band [min_score, max_score]
            t_inf_0 = time.perf_counter()
            candidates = self.prover.generate_candidate_actions(
                goal=active_subgoal,
                params=params,
                num_candidates=max(self.beam_width * 2, 8)
            )
            inf_time = (time.perf_counter() - t_inf_0) * 1000.0
            total_inf_time_ms += inf_time

            # Filter candidates within score band [min_score, max_score]
            filtered_candidates = [
                (tac, prior) for (tac, prior) in candidates
                if self.min_score <= prior <= self.max_score
            ]

            # If filtered list is empty, take top-k to prevent premature dead-end
            if not filtered_candidates:
                filtered_candidates = candidates[:self.beam_width]
            else:
                filtered_candidates = filtered_candidates[:self.beam_width]

            if not filtered_candidates:
                termination_reason = "no_candidates_generated"
                break

            # 2. Competitive Beam Evaluation & Milestone Detection
            decomp_rank = 4.5 if ("∧" in active_subgoal or "↔" in active_subgoal) else 1
            milestone_rank = {
                MilestoneKind.FULL_QED: 5,
                MilestoneKind.DECOMPOSITION: decomp_rank,
                MilestoneKind.SUBGOAL_CLOSED: 4,
                MilestoneKind.LEMMA_PROVED: 3,
                MilestoneKind.SIMPLIFICATION: 2,
            }
            candidate_milestones: List[Tuple[MilestoneCommit, List[str]]] = []
            failing_candidates_count = 0

            for c_idx, (tac, prior) in enumerate(filtered_candidates):
                total_expansions += 1
                total_simulations += 1

                t_step_0 = time.perf_counter()
                step_res = repl.step_tactic(curr_ps_id, tac)
                step_time = (time.perf_counter() - t_step_0) * 1000.0
                total_lean_time_ms += step_time

                effective_tac = tac

                # If tactic contained <;> and failed, try the prefix tactic (e.g. constructor) alone:
                if (
                    (step_res is None or step_res.get("error") is not None or (not step_res.get("is_solved") and step_res.get("new_proof_state") is None))
                    and "<;>" in tac
                ):
                    prefix_tac = tac.split("<;>")[0].strip()
                    if prefix_tac:
                        t_pref_0 = time.perf_counter()
                        step_res_pref = repl.step_tactic(curr_ps_id, prefix_tac)
                        total_lean_time_ms += (time.perf_counter() - t_pref_0) * 1000.0
                        total_simulations += 1
                        total_expansions += 1
                        if step_res_pref and step_res_pref.get("error") is None and step_res_pref.get("new_proof_state") is not None:
                            step_res = step_res_pref
                            effective_tac = prefix_tac

                # If tactic is a 'have' lemma and failed, try instantiating with context hypotheses:
                if (
                    (step_res is None or step_res.get("error") is not None or (not step_res.get("is_solved") and step_res.get("new_proof_state") is None))
                    and tac.strip().startswith("have ")
                ):
                    context_hyps = []
                    for line in active_subgoal.split("\n"):
                        if " : " in line and not line.strip().startswith("⊢"):
                            h_name = line.split(" : ")[0].strip()
                            if h_name and not any(c in h_name for c in " \t\n()[]{}.,:;+-*/=<>"):
                                context_hyps.append(h_name)
                    for h_name in context_hyps:
                        grounded_tac = f"{tac.strip()} {h_name}"
                        t_grd_0 = time.perf_counter()
                        step_res_grd = repl.step_tactic(curr_ps_id, grounded_tac)
                        total_lean_time_ms += (time.perf_counter() - t_grd_0) * 1000.0
                        total_simulations += 1
                        total_expansions += 1
                        if step_res_grd and step_res_grd.get("error") is None and step_res_grd.get("new_proof_state") is not None:
                            step_res = step_res_grd
                            effective_tac = grounded_tac
                            break

                if (time.perf_counter() - t_global_start) * 1000.0 >= budget_ms:
                    termination_reason = "budget_exhausted"
                    break

                if (
                    step_res is None
                    or step_res.get("error") is not None
                    or (not step_res.get("is_solved") and step_res.get("new_proof_state") is None)
                ):
                    failing_candidates_count += 1
                    continue

                if effective_tac in committed_script_steps:
                    continue

                new_ps_id = step_res.get("new_proof_state")
                new_subgoals = step_res.get("goals", [])
                is_solved = step_res.get("is_solved", False)

                # Check Milestone 1: FULL QED
                if is_solved or len(new_subgoals) == 0:
                    commit = MilestoneCommit(
                        step_index=recursion_depth,
                        tactic=effective_tac,
                        kind=MilestoneKind.FULL_QED,
                        subgoals_before=subgoals_before_count,
                        subgoals_after=0,
                        policy_prior=prior,
                        lean_latency_ms=step_time,
                        proof_state_id=new_ps_id if new_ps_id is not None else curr_ps_id,
                        active_goal_before=active_subgoal
                    )
                    candidate_milestones.append((commit, []))
                    break

                # Check Milestone 2: SUBGOAL REDUCTION (e.g. linarith closed a conjunct)
                if len(new_subgoals) < subgoals_before_count:
                    commit = MilestoneCommit(
                        step_index=recursion_depth,
                        tactic=effective_tac,
                        kind=MilestoneKind.SUBGOAL_CLOSED,
                        subgoals_before=subgoals_before_count,
                        subgoals_after=len(new_subgoals),
                        policy_prior=prior,
                        lean_latency_ms=step_time,
                        proof_state_id=new_ps_id,
                        active_goal_before=active_subgoal
                    )
                    candidate_milestones.append((commit, new_subgoals))
                    if prior >= 0.80:
                        break
                    continue

                # Check Milestone 3: HYPOTHESIS ENRICHMENT (kernel-verified non-trivial lemma)
                if effective_tac.strip().startswith("have ") and new_subgoals:
                    if len(new_subgoals) <= subgoals_before_count:
                        # Case A: Lemma is verified immediately by term or attached tactic
                        commit = MilestoneCommit(
                            step_index=recursion_depth,
                            tactic=effective_tac,
                            kind=MilestoneKind.LEMMA_PROVED,
                            subgoals_before=subgoals_before_count,
                            subgoals_after=len(new_subgoals),
                            policy_prior=prior,
                            lean_latency_ms=step_time,
                            proof_state_id=new_ps_id,
                            active_goal_before=active_subgoal
                        )
                        candidate_milestones.append((commit, new_subgoals))
                        if prior >= 0.85:
                            break
                        continue
                    elif len(new_subgoals) == subgoals_before_count + 1:
                        # Case B: Bare have statement opened an unproven lemma subgoal.
                        # Attempt to prove the lemma subgoal using standard solvers and active hypotheses:
                        context_hyps = []
                        for line in active_subgoal.split("\n"):
                            if " : " in line and not line.strip().startswith("⊢"):
                                h_name = line.split(" : ")[0].strip()
                                if h_name and not any(c in h_name for c in " \t\n()[]{}.,:;+-*/=<>"):
                                    context_hyps.append(h_name)

                        lemma_solvers = ["ring", "linarith", "nlinarith", "norm_num"]
                        for h_name in reversed(context_hyps):
                            lemma_solvers.append(f"linarith [{h_name}]")
                            lemma_solvers.append(f"nlinarith [{h_name}]")

                        if len(context_hyps) >= 2:
                            recent2 = ", ".join(context_hyps[-2:])
                            lemma_solvers.append(f"nlinarith [{recent2}]")
                            lemma_solvers.append(f"linarith [{recent2}]")
                        if len(context_hyps) >= 3:
                            recent3 = ", ".join(context_hyps[-3:])
                            lemma_solvers.append(f"nlinarith [{recent3}]")

                        ne_hyps = []
                        eq_hyps = []
                        for line in active_subgoal.split("\n"):
                            if " : " in line and not line.strip().startswith("⊢"):
                                h_name = line.split(" : ")[0].strip()
                                if h_name and not any(c in h_name for c in " \t\n()[]{}.,:;+-*/=<>"):
                                    if "≠ 0" in line:
                                        ne_hyps.append(h_name)
                                    if "=" in line:
                                        eq_hyps.append(h_name)

                        # Algebraic division cancellation via Mathlib div_eq_iff
                        for h_ne in ne_hyps:
                            for h_s in reversed(eq_hyps):
                                lemma_solvers.append(f"exact (div_eq_iff {h_ne}).mpr (by linarith [{h_s}])")
                                lemma_solvers.append(f"exact (div_eq_iff {h_ne}).mpr (by nlinarith [{h_s}])")

                        lemma_closed = False
                        for solver in lemma_solvers:
                            t_sol_0 = time.perf_counter()
                            sol_res = repl.step_tactic(new_ps_id, solver)
                            sol_time = (time.perf_counter() - t_sol_0) * 1000.0
                            total_lean_time_ms += sol_time
                            total_simulations += 1
                            total_expansions += 1
                            if sol_res and sol_res.get("error") is None and sol_res.get("new_proof_state") is not None:
                                sol_subgoals = sol_res.get("goals", [])
                                if len(sol_subgoals) <= subgoals_before_count:
                                    lemma_closed = True
                                    effective_tac = f"{effective_tac.strip()} := by {solver}"
                                    new_ps_id = sol_res.get("new_proof_state")
                                    new_subgoals = sol_subgoals
                                    step_time += sol_time
                                    break
                        if lemma_closed:
                            commit = MilestoneCommit(
                                step_index=recursion_depth,
                                tactic=effective_tac,
                                kind=MilestoneKind.LEMMA_PROVED,
                                subgoals_before=subgoals_before_count,
                                subgoals_after=len(new_subgoals),
                                policy_prior=prior,
                                lean_latency_ms=step_time,
                                proof_state_id=new_ps_id,
                                active_goal_before=active_subgoal
                            )
                            candidate_milestones.append((commit, new_subgoals))
                            if prior >= 0.85:
                                break
                            continue

                # Check Milestone 4: DECOMPOSITION (structural introduction/splitting of compound goal)
                if (
                    effective_tac.strip() in ("constructor", "intro", "rintro", "ext", "funext", "split")
                    and new_subgoals
                    and new_ps_id is not None
                ):
                    commit = MilestoneCommit(
                        step_index=recursion_depth,
                        tactic=effective_tac,
                        kind=MilestoneKind.DECOMPOSITION,
                        subgoals_before=subgoals_before_count,
                        subgoals_after=len(new_subgoals),
                        policy_prior=prior,
                        lean_latency_ms=step_time,
                        proof_state_id=new_ps_id,
                        active_goal_before=active_subgoal
                    )
                    candidate_milestones.append((commit, new_subgoals))
                    if decomp_rank > 4 or prior >= 0.85:
                        break
                    continue

                # Check Milestone 5: SIMPLIFICATION (kernel-verified rewrite advancing state)
                if (
                    effective_tac.strip().startswith("rw ")
                    and new_subgoals
                    and new_ps_id is not None
                    and new_subgoals != [active_subgoal]
                ):
                    commit = MilestoneCommit(
                        step_index=recursion_depth,
                        tactic=effective_tac,
                        kind=MilestoneKind.SIMPLIFICATION,
                        subgoals_before=subgoals_before_count,
                        subgoals_after=len(new_subgoals),
                        policy_prior=prior,
                        lean_latency_ms=step_time,
                        proof_state_id=new_ps_id,
                        active_goal_before=active_subgoal
                    )
                    candidate_milestones.append((commit, new_subgoals))
                    if prior >= 0.80:
                        break
                    continue

            # 3. Hard Commit or Local Tree Step
            milestone_found = False
            best_milestone = None
            if candidate_milestones:
                best_milestone, winning_subgoals = max(
                    candidate_milestones,
                    key=lambda item: (milestone_rank[item[0].kind], item[0].policy_prior)
                )
                milestone_found = True
                trees_destroyed += (len(filtered_candidates) - 1)
                committed_script_steps.append(best_milestone.tactic)
                commits.append(best_milestone)
                curr_ps_id = best_milestone.proof_state_id
                curr_subgoals = winning_subgoals

                if best_milestone.kind == MilestoneKind.FULL_QED:
                    termination_reason = "solved"
                    break
            else:
                # If no immediate 1-step milestone occurred, attempt local shallow MCTS on the top candidates
                shallow_milestone = self._explore_shallow_milestone(
                    candidates=filtered_candidates,
                    curr_ps_id=curr_ps_id,
                    subgoals_before=subgoals_before_count,
                    raw_code=raw_code,
                    t_global_start=t_global_start,
                    budget_ms=budget_ms,
                )

                if shallow_milestone is not None:
                    m_commit, add_lean, add_inf, add_exp, add_sims, add_destroyed = shallow_milestone
                    total_lean_time_ms += add_lean
                    total_inf_time_ms += add_inf
                    total_expansions += add_exp
                    total_simulations += add_sims
                    trees_destroyed += add_destroyed

                    committed_script_steps.append(m_commit.tactic)
                    commits.append(m_commit)
                    curr_ps_id = m_commit.proof_state_id
                    curr_subgoals = []
                    if repl is not None:
                        # Fetch subgoals from current state
                        chk_step = repl.step_tactic(curr_ps_id, "skip")
                        if chk_step and "goals" in chk_step:
                            curr_subgoals = chk_step.get("goals", [])

                    if m_commit.kind == MilestoneKind.FULL_QED:
                        termination_reason = "solved"
                        break
                else:
                    # Neither 1-step nor shallow 2-step produced an approved milestone
                    if (time.perf_counter() - t_global_start) * 1000.0 >= budget_ms:
                        termination_reason = "budget_exhausted"
                    else:
                        termination_reason = "frontier_stalled_no_milestones"
                    break

        total_elapsed_ms = (time.perf_counter() - t_global_start) * 1000.0
        is_solved = (termination_reason == "solved")
        winning_script = "\n".join(committed_script_steps) if is_solved else None

        # Verify kernel validity if solved
        if is_solved and winning_script:
            thm_name = pid.split("/")[-1] if "/" in pid else pid
            stmt_clean = repl._prepare_statement(raw_code)
            tokens = stmt_clean.split()
            for idx, tok in enumerate(tokens):
                if tok in ("theorem", "lemma", "def") and idx + 1 < len(tokens):
                    candidate_name = tokens[idx + 1].split("(")[0].split("{")[0].strip()
                    if candidate_name.isidentifier():
                        thm_name = candidate_name
                        break

            script_lines = []
            for step in committed_script_steps:
                for line in step.split("\n"):
                    if line.strip():
                        script_lines.append(f"  {line.strip()}")
            full_code = f"{stmt_clean} := by\n" + "\n".join(script_lines)
            ok, _, err_msg, axioms = repl.verify_full_code(full_code, thm_name)
            if not ok or "sorryAx" in axioms:
                is_solved = False
                winning_script = None
                termination_reason = f"kernel_verification_failed: {err_msg}"

        return RMBCSearchResult(
            problem_id=pid,
            solved=is_solved,
            winning_script=winning_script,
            total_aggregate_time_ms=total_elapsed_ms,
            total_lean_time_ms=total_lean_time_ms,
            total_inference_time_ms=total_inf_time_ms,
            total_node_expansions=total_expansions,
            total_simulations=total_simulations,
            trees_destroyed_count=trees_destroyed,
            milestone_commits=[
                {
                    "step_index": c.step_index,
                    "tactic": c.tactic,
                    "kind": c.kind.value,
                    "subgoals_before": c.subgoals_before,
                    "subgoals_after": c.subgoals_after,
                    "prior": c.policy_prior,
                    "latency_ms": c.lean_latency_ms,
                }
                for c in commits
            ],
            termination_reason=termination_reason,
        )

    def _explore_shallow_milestone(
        self,
        candidates: List[Tuple[str, float]],
        curr_ps_id: int,
        subgoals_before: int,
        raw_code: str,
        t_global_start: float,
        budget_ms: float,
    ) -> Optional[Tuple[MilestoneCommit, float, float, int, int, int]]:
        """
        Shallow 2-step competitive rollout to find subgoals/lemmas that require 2 tactic steps.
        """
        repl = self.prover.repl_client
        add_lean = 0.0
        add_inf = 0.0
        add_exp = 0
        add_sims = 0
        trees_destroyed = 0

        for c_idx, (tac1, prior1) in enumerate(candidates):
            if (time.perf_counter() - t_global_start) * 1000.0 >= budget_ms:
                break

            t0 = time.perf_counter()
            r1 = repl.step_tactic(curr_ps_id, tac1)
            add_lean += (time.perf_counter() - t0) * 1000.0
            add_sims += 1
            add_exp += 1

            if (
                r1 is None
                or r1.get("error") is not None
                or (not r1.get("is_solved") and r1.get("new_proof_state") is None)
            ):
                continue

            ps1 = r1.get("new_proof_state")
            goals1 = r1.get("goals", [])
            is_solved1 = r1.get("is_solved", False)
            if is_solved1 or not goals1:
                # 1st step closed it!
                trees_destroyed += (len(candidates) - 1)
                commit = MilestoneCommit(
                    step_index=1,
                    tactic=tac1,
                    kind=MilestoneKind.FULL_QED,
                    subgoals_before=subgoals_before,
                    subgoals_after=0,
                    policy_prior=prior1,
                    lean_latency_ms=add_lean,
                    proof_state_id=ps1 if ps1 is not None else curr_ps_id,
                    active_goal_before="",
                )
                return commit, add_lean, add_inf, add_exp, add_sims, trees_destroyed

            # Query model for step 2
            t_inf = time.perf_counter()
            step2_cands = self.prover.generate_candidate_actions(goals1[0], "", num_candidates=3)
            add_inf += (time.perf_counter() - t_inf) * 1000.0

            for tac2, prior2 in step2_cands:
                t0_2 = time.perf_counter()
                r2 = repl.step_tactic(ps1, tac2)
                add_lean += (time.perf_counter() - t0_2) * 1000.0
                add_sims += 1
                add_exp += 1

                if (
                    r2 is None
                    or r2.get("error") is not None
                    or (not r2.get("is_solved") and r2.get("new_proof_state") is None)
                ):
                    continue

                ps2 = r2.get("new_proof_state")
                goals2 = r2.get("goals", [])
                is_solved2 = r2.get("is_solved", False)

                combined_tac = f"{tac1}\n{tac2}"

                if is_solved2 or len(goals2) == 0:
                    trees_destroyed += (len(candidates) - 1)
                    commit = MilestoneCommit(
                        step_index=1,
                        tactic=combined_tac,
                        kind=MilestoneKind.FULL_QED,
                        subgoals_before=subgoals_before,
                        subgoals_after=0,
                        policy_prior=prior1 * prior2,
                        lean_latency_ms=add_lean,
                        proof_state_id=ps2 if ps2 is not None else ps1,
                        active_goal_before="",
                    )
                    return commit, add_lean, add_inf, add_exp, add_sims, trees_destroyed

                if len(goals2) < subgoals_before:
                    trees_destroyed += (len(candidates) - 1)
                    commit = MilestoneCommit(
                        step_index=1,
                        tactic=combined_tac,
                        kind=MilestoneKind.SUBGOAL_CLOSED,
                        subgoals_before=subgoals_before,
                        subgoals_after=len(goals2),
                        policy_prior=prior1 * prior2,
                        lean_latency_ms=add_lean,
                        proof_state_id=ps2,
                        active_goal_before="",
                    )
                    return commit, add_lean, add_inf, add_exp, add_sims, trees_destroyed

        return None
