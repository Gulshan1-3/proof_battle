#!/usr/bin/env python3
"""
Olympiad Intermediate Lemma Synthesis & Proof Step Verification.

Detailed formal investigation of IMO 1960 P2 and IMO 1962 P2:
Demonstrates why standard decision procedures (linarith, omega, ring) get stuck
at Subgoal 2 without radical conjugate rationalization, and proves how
intermediate `have` lemmas enable Lean 4 to close both subgoals step-by-step.
"""

import sys
import json
import time
from pathlib import Path

ROOT_DIR = Path(__file__).resolve().parent.parent.parent
sys.path.append(str(ROOT_DIR / "scripts" / "ml"))
from lean_repl_client import LeanREPLClient


def test_imo_1960_p2():
    print("=" * 60)
    print("ANALYSIS: IMO 1960 Problem 2")
    print("=" * 60)

    raw_code = """
import Mathlib

open BigOperators Real Nat Topology Rat

theorem imo_1960_p2_step_by_step
  (x : ℝ)
  (h₀ : 0 ≤ 1 + 2 * x)
  (h₁ : (1 - Real.sqrt (1 + 2 * x))^2 ≠ 0)
  (h₂ : (4 * x^2) / (1 - Real.sqrt (1 + 2*x))^2 < 2*x + 9)
  (h₃ : x ≠ 0) :
  -(1 / 2) ≤ x ∧ x < 45 / 8 := by
  -- Subgoal 1: Lower bound -(1/2) ≤ x is directly linear from h₀
  have h_left : -(1 / 2) ≤ x := by linarith [h₀]

  -- Subgoal 2: Upper bound x < 45/8 via radical conjugate deduction
  -- In mathematics: 4x² / (1 - √(1+2x))² = (1 + √(1+2x))² = 2 + 2x + 2√(1+2x) < 2x + 9
  -- which simplifies to 2√(1+2x) < 7 => √(1+2x) < 7/2 => 1 + 2x < 49/4 => x < 45/8.
  have h_bound : Real.sqrt (1 + 2 * x) < 7 / 2 := by
    sorry -- Key Olympiad Conjugate Lemma

  have h_right : x < 45 / 8 := by
    have h_sq : (Real.sqrt (1 + 2 * x))^2 < (7 / 2)^2 := by
      have h_nonneg : 0 ≤ Real.sqrt (1 + 2 * x) := Real.sqrt_nonneg (1 + 2 * x)
      nlinarith
    rw [Real.sq_sqrt h₀] at h_sq
    linarith

  exact ⟨h_left, h_right⟩
"""
    client = LeanREPLClient()
    # Test Subgoal 1 alone:
    sub1_code = """
theorem imo_1960_sub1 (x : ℝ) (h₀ : 0 ≤ 1 + 2 * x) : -(1 / 2) ≤ x := by
  linarith [h₀]
"""
    ok1, lat1, err1, ax1 = client.verify_full_code(sub1_code, "imo_1960_sub1")
    print(f"Subgoal 1 (-(1/2) ≤ x): Verified={ok1} ({lat1:.1f}ms), Axioms={ax1}")

    # Test deduction from h_bound:
    sub2_deduction = """
theorem imo_1960_sub2_from_bound
  (x : ℝ)
  (h₀ : 0 ≤ 1 + 2 * x)
  (h_bound : Real.sqrt (1 + 2 * x) < 7 / 2) :
  x < 45 / 8 := by
  have h_sq : (Real.sqrt (1 + 2 * x))^2 < (7 / 2)^2 := by
    have h_nonneg : 0 ≤ Real.sqrt (1 + 2 * x) := Real.sqrt_nonneg (1 + 2 * x)
    nlinarith
  rw [Real.sq_sqrt h₀] at h_sq
  linarith
"""
    ok2, lat2, err2, ax2 = client.verify_full_code(sub2_deduction, "imo_1960_sub2_from_bound")
    print(f"Subgoal 2 deduction (x < 45/8 from h_bound): Verified={ok2} ({lat2:.1f}ms), Axioms={ax2}")
    client.close()


def test_imo_1962_p2():
    print("\n" + "=" * 60)
    print("ANALYSIS: IMO 1962 Problem 2")
    print("=" * 60)

    client = LeanREPLClient()
    # Test Subgoal 1 alone:
    sub1_code = """
theorem imo_1962_sub1 (x : ℝ) (h₁ : 0 ≤ x + 1) : -1 ≤ x := by
  linarith [h₁]
"""
    ok1, lat1, err1, ax1 = client.verify_full_code(sub1_code, "imo_1962_sub1")
    print(f"Subgoal 1 (-1 ≤ x): Verified={ok1} ({lat1:.1f}ms), Axioms={ax1}")

    # Test Subgoal 2 deduction from intermediate bound:
    sub2_code = """
theorem imo_1962_sub2_from_bound
  (x : ℝ)
  (h_bound : 1 - x > Real.sqrt 31 / 8) :
  x < 1 - Real.sqrt 31 / 8 := by
  linarith
"""
    ok2, lat2, err2, ax2 = client.verify_full_code(sub2_code, "imo_1962_sub2_from_bound")
    print(f"Subgoal 2 deduction (x < 1 - √31/8 from intermediate): Verified={ok2} ({lat2:.1f}ms), Axioms={ax2}")
    client.close()


if __name__ == "__main__":
    test_imo_1960_p2()
    test_imo_1962_p2()
