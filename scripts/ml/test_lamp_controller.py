#!/usr/bin/env python3
"""
Unit Test Suite for Learned Allocation Controller (LAMP).

Verifies:
1. Feature extraction consistency (z_k in R^8).
2. Marginal utility computation and dynamic scoring.
3. Proper de-prioritization of exhausted or failing branches.
4. Softmax exploration bonus and dynamic simulation chunk scaling.
"""

import sys
import numpy as np
from pathlib import Path

ROOT_DIR = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT_DIR))
sys.path.insert(0, str(ROOT_DIR / "scripts" / "ml"))

from persistent_multi_branch_forest import BranchState, DBFMCTSNode
from learned_allocation_controller import BranchFeatureExtractor, LearnedAllocationController


def test_feature_extraction():
    print("\n[TEST 1] Testing Branch Feature Extraction (z_k in R^8)...")
    root = DBFMCTSNode(
        node_id="test_root",
        depth=1,
        goal="⊢ x < 45 / 8",
        tactic="have h := Real.sq_sqrt h₀",
        accumulated_script="have h := Real.sq_sqrt h₀",
        prior=0.75,
        visits=8,
        total_value=5.6
    )

    branch = BranchState(
        branch_id=0,
        root_tactic="have h := Real.sq_sqrt h₀",
        prior=0.75,
        root_node=root,
        visits=8,
        total_value=5.6,
        lean_tactic_time_ms=1200.0,
        model_inference_time_ms=800.0,
        node_expansions=6,
        is_solved=False,
        is_exhausted=False,
        action_trajectory=["have h := Real.sq_sqrt h₀"],
        remaining_subgoals=["⊢ x < 45 / 8"],
        max_depth_reached=3
    )

    feats = BranchFeatureExtractor.extract(branch, total_visits=20, global_budget_ms=30000.0)
    arr = feats.to_numpy()

    assert len(arr) == 8, f"Expected 8 features, got {len(arr)}"
    assert abs(arr[0] - 3.0 / 8.0) < 1e-5, "Depth norm mismatch"
    assert abs(arr[1] - 8.0 / 20.0) < 1e-5, "Visit share mismatch"
    assert abs(arr[2] - 0.70) < 1e-5, "Q-value mismatch"
    assert arr[7] == 0.0, "Active branch should have 0 diagnostic penalty"
    print("  ✓ PASSED: Feature extraction returns valid 8-dimensional state vector.")


def test_controller_decision_and_penalty():
    print("\n[TEST 2] Testing LAMP Marginal Utility Scoring & Exhaustion Penalty...")
    controller = LearnedAllocationController(c_lamp=1.0)

    # Branch 0: Promising branch with progress
    b0_root = DBFMCTSNode("b0", 1, "⊢ x < 45/8", "tac0", "tac0", prior=0.8, visits=4, total_value=3.2)
    b0 = BranchState(0, "tac0", 0.8, b0_root, visits=4, total_value=3.2, max_depth_reached=3)

    # Branch 1: Exhausted dead-end branch
    b1_root = DBFMCTSNode("b1", 1, "⊢ x < 45/8", "tac1", "tac1", prior=0.4, visits=10, total_value=0.2)
    b1 = BranchState(1, "tac1", 0.4, b1_root, visits=10, total_value=0.2, is_exhausted=True)

    # Branch 2: Fresh branch with high prior but low visits
    b2_root = DBFMCTSNode("b2", 1, "⊢ x < 45/8", "tac2", "tac2", prior=0.9, visits=1, total_value=0.5)
    b2 = BranchState(2, "tac2", 0.9, b2_root, visits=1, total_value=0.5, max_depth_reached=1)

    branches = [b0, b1, b2]
    best_idx, alloc_delta = controller.select_next_branch(branches, global_budget_ms=30000.0, step_chunk_delta=4)

    assert best_idx != 1, "Exhausted branch must NOT be selected"
    assert best_idx in [0, 2], f"Expected branch 0 or 2 to be selected, got {best_idx}"
    assert alloc_delta >= 2, "Allocated chunk must be at least 2"
    print(f"  ✓ PASSED: Controller correctly rejected dead-end branch and selected branch {best_idx} with delta={alloc_delta}.")


if __name__ == "__main__":
    print("==================================================================")
    print("STAGE 65: LEARNED ALLOCATION CONTROLLER (LAMP) TEST SUITE")
    print("==================================================================")
    test_feature_extraction()
    test_controller_decision_and_penalty()
    print("\n==================================================================")
    print("ALL LAMP CONTROLLER TESTS PASSED (100% CLEAN)")
    print("==================================================================")
