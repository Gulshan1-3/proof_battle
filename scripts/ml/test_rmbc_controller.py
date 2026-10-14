#!/usr/bin/env python3
"""
Unit and Integration Test Suite for RMBC Search Controller.
"""

import sys
import unittest
from pathlib import Path

ROOT_DIR = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT_DIR))
sys.path.insert(0, str(ROOT_DIR / "scripts" / "ml"))

from dbf_mcts_prover import DBFMCTSProver
from rmbc_controller import (
    RecursiveMilestoneBeamCommitProver,
    MilestoneKind,
    MilestoneCommit,
    RMBCSearchResult,
)


class TestRMBCController(unittest.TestCase):

    def test_milestone_kind_and_commit(self):
        commit = MilestoneCommit(
            step_index=1,
            tactic="constructor <;> linarith",
            kind=MilestoneKind.SUBGOAL_CLOSED,
            subgoals_before=2,
            subgoals_after=1,
            policy_prior=0.78,
            lean_latency_ms=45.2,
            proof_state_id=102,
            active_goal_before="⊢ -(1/2) ≤ x ∧ x < 45/8",
        )
        self.assertEqual(commit.kind, MilestoneKind.SUBGOAL_CLOSED)
        self.assertEqual(commit.subgoals_before, 2)
        self.assertEqual(commit.subgoals_after, 1)

    def test_confidence_filtering(self):
        prover = DBFMCTSProver(use_repl=False)
        rmbc = RecursiveMilestoneBeamCommitProver(
            prover=prover,
            min_score=0.40,
            max_score=0.80,
            beam_width=5
        )

        # Mock candidates
        raw_candidates = [
            ("tactic_high", 0.95),   # > 0.80
            ("tactic_mid1", 0.78),   # In band
            ("tactic_mid2", 0.65),   # In band
            ("tactic_mid3", 0.45),   # In band
            ("tactic_low", 0.20),    # < 0.40
        ]

        filtered = [c for c in raw_candidates if rmbc.min_score <= c[1] <= rmbc.max_score]
        self.assertEqual(len(filtered), 3)
        self.assertEqual(filtered[0][0], "tactic_mid1")
        self.assertEqual(filtered[1][0], "tactic_mid2")
        self.assertEqual(filtered[2][0], "tactic_mid3")

    def test_rmbc_end_to_end_on_valid_problem(self):
        prover = DBFMCTSProver(use_repl=True)
        rmbc = RecursiveMilestoneBeamCommitProver(
            prover=prover,
            min_score=0.30,
            max_score=0.90,
            beam_width=5
        )

        test_problem = {
            "problem_id": "valid/algebra_2rootsintpoly_am10tap11eqasqpam110",
            "goal": "∀ a : ℝ, (a - 10) * (a + 11) = a^2 + a - 110",
            "params": "a : ℝ",
            "raw_code": "import Mathlib\n\ntheorem algebra_2rootsintpoly_am10tap11eqasqpam110 (a : ℝ) : (a - 10) * (a + 11) = a^2 + a - 110 := by sorry"
        }

        res = rmbc.search(test_problem, aggregate_budget_sec=15.0)
        prover.close()

        self.assertTrue(res.solved, f"Failed with: {res.termination_reason}")
        self.assertIsNotNone(res.winning_script)
        self.assertIn("ring", res.winning_script.lower())
        self.assertGreaterEqual(len(res.milestone_commits), 1)
        self.assertEqual(res.milestone_commits[0]["kind"], MilestoneKind.FULL_QED.value)
        print(f"\n[RMBC Test] Successfully solved {test_problem['problem_id']} in {res.total_aggregate_time_ms:.1f}ms")
        print(f"[RMBC Test] Winning script:\n{res.winning_script}")


if __name__ == "__main__":
    unittest.main()
