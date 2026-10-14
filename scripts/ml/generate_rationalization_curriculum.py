#!/usr/bin/env python3
"""
Procedural Curriculum Generator for Algebraic Rationalization Traces.

Generates verified multi-step training traces for radical difference-of-squares
rationalization with randomized variables (x, y, z, u, w) and randomized
coefficients (b in [2, 10], K in [5, 25]).

Each problem yields 10 atomic intermediate state-action pairs:
1. `constructor` (conjunction decomposition)
2. `linarith [h₀]` / `nlinarith` (subgoal 1 lower bound)
3. `have h := Real.sq_sqrt h₀` (radical square lemma)
4. `have h_prod : (1 - Real.sqrt (1 + b*v)) * (1 + Real.sqrt (1 + b*v)) = - (b * v)` (conjugate product)
5. `have h_sq : (1 - Real.sqrt (1 + b*v))^2 * (1 + Real.sqrt (1 + b*v))^2 = b^2 * v^2` (squared conjugate product)
6. `have h_div : (b^2 * v^2) / (1 - Real.sqrt (1 + b*v))^2 = (1 + Real.sqrt (1 + b*v))^2` (cancellation)
7. `rw [h_div] at h₂` (rewrite in inequality)
8. `have h_exp : (1 + Real.sqrt (1 + b*v))^2 = 2 + b * v + 2 * Real.sqrt (1 + b * v)` (square expansion)
9. `rw [h_exp] at h₂` (rewrite expanded)
10. `have h_rad : Real.sqrt (1 + b * v) < ...` / `linarith` (isolation & monotonicity)
"""

import sys
import json
import random
import argparse
from pathlib import Path
from fractions import Fraction
from typing import List, Dict, Any, Tuple, Optional

ROOT_DIR = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT_DIR))
sys.path.insert(0, str(ROOT_DIR / "scripts" / "ml"))

from lean_repl_client import LeanREPLClient

DATA_DIR = ROOT_DIR / "data" / "curriculum"
DATA_DIR.mkdir(parents=True, exist_ok=True)
OUT_FILE = DATA_DIR / "algebraic_rationalization_traces.jsonl"


VARS = ["x", "y", "z", "u", "w"]
B_COEFFS = [2, 3, 4, 5, 6, 7, 8, 9, 10]
K_CONSTS = [5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25]


def format_frac(frac: Fraction) -> str:
    """Formats a Fraction as a Lean 4 real literal string."""
    if frac.denominator == 1:
        return str(frac.numerator)
    return f"({frac.numerator} / {frac.denominator} : ℝ)" if frac.numerator < 0 else f"{frac.numerator} / {frac.denominator}"


def compute_bounds(b: int, K: int) -> Tuple[Fraction, Fraction]:
    """Computes exact lower bound -(1/b) and upper bound for v."""
    lower_bound = Fraction(-1, b)
    rad_bound = Fraction(K - 2, 2)
    upper_bound = Fraction(1, b) * (rad_bound ** 2 - 1)
    return lower_bound, upper_bound


def generate_single_problem(v: str, b: int, K: int, thm_id: int) -> Dict[str, Any]:
    """Generates complete Lean 4 theorem code and decomposed training traces."""
    b_sq = b * b
    lower_frac, upper_frac = compute_bounds(b, K)

    lower_str = f"-({Fraction(1, b).numerator} / {Fraction(1, b).denominator} : ℝ)" if b > 1 else "-1"
    upper_str = format_frac(upper_frac)
    rad_bound_str = format_frac(Fraction(K - 2, 2))

    thm_name = f"synth_rationalization_{thm_id}_{v}_b{b}_k{K}"

    raw_code = f"""import Mathlib

open Real

theorem {thm_name}
  ({v} : ℝ)
  (h₀ : 0 ≤ 1 + {b} * {v})
  (h₁ : (1 - Real.sqrt (1 + {b} * {v}))^2 ≠ 0)
  (h₂ : ({b_sq} * {v}^2) / (1 - Real.sqrt (1 + {b} * {v}))^2 < {b} * {v} + {K})
  (h₃ : {v} ≠ 0) :
  {lower_str} ≤ {v} ∧ {v} < {upper_str} := by sorry
"""

    # Full verified proof script
    proof_script = f"""constructor
· linarith [h₀]
· have h : Real.sqrt (1 + {b} * {v})^2 = 1 + {b} * {v} := Real.sq_sqrt h₀
  have h_prod : (1 - Real.sqrt (1 + {b} * {v})) * (1 + Real.sqrt (1 + {b} * {v})) = - ({b} * {v}) := by
    have : (1 - Real.sqrt (1 + {b} * {v})) * (1 + Real.sqrt (1 + {b} * {v})) = 1 - Real.sqrt (1 + {b} * {v})^2 := by ring
    rw [this, h]
    ring
  have h_sq : (1 - Real.sqrt (1 + {b} * {v}))^2 * (1 + Real.sqrt (1 + {b} * {v}))^2 = {b_sq} * {v}^2 := by
    calc
      (1 - Real.sqrt (1 + {b} * {v}))^2 * (1 + Real.sqrt (1 + {b} * {v}))^2 = ((1 - Real.sqrt (1 + {b} * {v})) * (1 + Real.sqrt (1 + {b} * {v})))^2 := by ring
      _ = (- ({b} * {v}))^2 := by rw [h_prod]
      _ = {b_sq} * {v}^2 := by ring
  have h_div : ({b_sq} * {v}^2) / (1 - Real.sqrt (1 + {b} * {v}))^2 = (1 + Real.sqrt (1 + {b} * {v}))^2 := by
    rw [← h_sq]
    exact mul_div_cancel_left₀ ((1 + Real.sqrt (1 + {b} * {v}))^2) h₁
  rw [h_div] at h₂
  have h_exp : (1 + Real.sqrt (1 + {b} * {v}))^2 = 2 + {b} * {v} + 2 * Real.sqrt (1 + {b} * {v}) := by
    calc
      (1 + Real.sqrt (1 + {b} * {v}))^2 = 1 + 2 * Real.sqrt (1 + {b} * {v}) + Real.sqrt (1 + {b} * {v})^2 := by ring
      _ = 1 + 2 * Real.sqrt (1 + {b} * {v}) + (1 + {b} * {v}) := by rw [h]
      _ = 2 + {b} * {v} + 2 * Real.sqrt (1 + {b} * {v}) := by ring
  rw [h_exp] at h₂
  have h_rad : Real.sqrt (1 + {b} * {v}) < {rad_bound_str} := by linarith
  have h_rad_pos : 0 ≤ Real.sqrt (1 + {b} * {v}) := Real.sqrt_nonneg (1 + {b} * {v})
  have h_sq_lt : (Real.sqrt (1 + {b} * {v}))^2 < ({rad_bound_str} : ℝ)^2 := by
    nlinarith [h_rad, h_rad_pos]
  rw [h] at h_sq_lt
  linarith
"""

    # Decomposed state-action training traces
    common_hyps = f"h₀ : 0 ≤ 1 + {b} * {v}, h₁ : (1 - Real.sqrt (1 + {b} * {v}))^2 ≠ 0, h₂ : ({b_sq} * {v}^2) / (1 - Real.sqrt (1 + {b} * {v}))^2 < {b} * {v} + {K}, h₃ : {v} ≠ 0"

    traces = [
        # Step 1: Conjunction decomposition
        {
            "hypotheses": common_hyps,
            "goal": f"{lower_str} ≤ {v} ∧ {v} < {upper_str}",
            "tactic": "constructor",
        },
        # Step 2: Lower bound
        {
            "hypotheses": common_hyps,
            "goal": f"{lower_str} ≤ {v}",
            "tactic": "linarith [h₀]",
        },
        # Step 3: Radical hypothesis
        {
            "hypotheses": common_hyps,
            "goal": f"{v} < {upper_str}",
            "tactic": "have h := Real.sq_sqrt h₀",
        },
        # Step 4: Conjugate product identity
        {
            "hypotheses": f"{common_hyps}, h : Real.sqrt (1 + {b} * {v})^2 = 1 + {b} * {v}",
            "goal": f"{v} < {upper_str}",
            "tactic": f"have h_prod : (1 - Real.sqrt (1 + {b} * {v})) * (1 + Real.sqrt (1 + {b} * {v})) = - ({b} * {v})",
        },
        # Step 5: Squared conjugate product
        {
            "hypotheses": f"{common_hyps}, h : Real.sqrt (1 + {b} * {v})^2 = 1 + {b} * {v}, h_prod : (1 - Real.sqrt (1 + {b} * {v})) * (1 + Real.sqrt (1 + {b} * {v})) = - ({b} * {v})",
            "goal": f"{v} < {upper_str}",
            "tactic": f"have h_sq : (1 - Real.sqrt (1 + {b} * {v}))^2 * (1 + Real.sqrt (1 + {b} * {v}))^2 = {b_sq} * {v}^2",
        },
        # Step 6: Denominator cancellation
        {
            "hypotheses": f"{common_hyps}, h : Real.sqrt (1 + {b} * {v})^2 = 1 + {b} * {v}, h_prod : (1 - Real.sqrt (1 + {b} * {v})) * (1 + Real.sqrt (1 + {b} * {v})) = - ({b} * {v}), h_sq : (1 - Real.sqrt (1 + {b} * {v}))^2 * (1 + Real.sqrt (1 + {b} * {v}))^2 = {b_sq} * {v}^2",
            "goal": f"{v} < {upper_str}",
            "tactic": f"have h_div : ({b_sq} * {v}^2) / (1 - Real.sqrt (1 + {b} * {v}))^2 = (1 + Real.sqrt (1 + {b} * {v}))^2",
        },
        # Step 7: Substitute cancellation into inequality
        {
            "hypotheses": f"{common_hyps}, h : Real.sqrt (1 + {b} * {v})^2 = 1 + {b} * {v}, h_prod : (1 - Real.sqrt (1 + {b} * {v})) * (1 + Real.sqrt (1 + {b} * {v})) = - ({b} * {v}), h_sq : (1 - Real.sqrt (1 + {b} * {v}))^2 * (1 + Real.sqrt (1 + {b} * {v}))^2 = {b_sq} * {v}^2, h_div : ({b_sq} * {v}^2) / (1 - Real.sqrt (1 + {b} * {v}))^2 = (1 + Real.sqrt (1 + {b} * {v}))^2",
            "goal": f"{v} < {upper_str}",
            "tactic": "rw [h_div] at h₂",
        },
        # Step 8: Radical expansion
        {
            "hypotheses": f"h₀ : 0 ≤ 1 + {b} * {v}, h₁ : (1 - Real.sqrt (1 + {b} * {v}))^2 ≠ 0, h₂ : (1 + Real.sqrt (1 + {b} * {v}))^2 < {b} * {v} + {K}, h₃ : {v} ≠ 0, h : Real.sqrt (1 + {b} * {v})^2 = 1 + {b} * {v}, h_prod : (1 - Real.sqrt (1 + {b} * {v})) * (1 + Real.sqrt (1 + {b} * {v})) = - ({b} * {v}), h_sq : (1 - Real.sqrt (1 + {b} * {v}))^2 * (1 + Real.sqrt (1 + {b} * {v}))^2 = {b_sq} * {v}^2, h_div : ({b_sq} * {v}^2) / (1 - Real.sqrt (1 + {b} * {v}))^2 = (1 + Real.sqrt (1 + {b} * {v}))^2",
            "goal": f"{v} < {upper_str}",
            "tactic": f"have h_exp : (1 + Real.sqrt (1 + {b} * {v}))^2 = 2 + {b} * {v} + 2 * Real.sqrt (1 + {b} * {v})",
        },
        # Step 9: Rewrite expanded radical
        {
            "hypotheses": f"h₀ : 0 ≤ 1 + {b} * {v}, h₁ : (1 - Real.sqrt (1 + {b} * {v}))^2 ≠ 0, h₂ : (1 + Real.sqrt (1 + {b} * {v}))^2 < {b} * {v} + {K}, h₃ : {v} ≠ 0, h : Real.sqrt (1 + {b} * {v})^2 = 1 + {b} * {v}, h_exp : (1 + Real.sqrt (1 + {b} * {v}))^2 = 2 + {b} * {v} + 2 * Real.sqrt (1 + {b} * {v})",
            "goal": f"{v} < {upper_str}",
            "tactic": "rw [h_exp] at h₂",
        },
        # Step 10: Linear isolation of radical
        {
            "hypotheses": f"h₀ : 0 ≤ 1 + {b} * {v}, h₁ : (1 - Real.sqrt (1 + {b} * {v}))^2 ≠ 0, h₂ : 2 + {b} * {v} + 2 * Real.sqrt (1 + {b} * {v}) < {b} * {v} + {K}, h₃ : {v} ≠ 0, h : Real.sqrt (1 + {b} * {v})^2 = 1 + {b} * {v}",
            "goal": f"{v} < {upper_str}",
            "tactic": f"have h_rad : Real.sqrt (1 + {b} * {v}) < {rad_bound_str} := by linarith",
        },
        # Step 11: Final solve via non-linear arithmetic
        {
            "hypotheses": f"h₀ : 0 ≤ 1 + {b} * {v}, h₁ : (1 - Real.sqrt (1 + {b} * {v}))^2 ≠ 0, h₂ : 2 + {b} * {v} + 2 * Real.sqrt (1 + {b} * {v}) < {b} * {v} + {K}, h₃ : {v} ≠ 0, h : Real.sqrt (1 + {b} * {v})^2 = 1 + {b} * {v}, h_rad : Real.sqrt (1 + {b} * {v}) < {rad_bound_str}",
            "goal": f"{v} < {upper_str}",
            "tactic": f"nlinarith [h_rad, Real.sqrt_nonneg _, h]",
        },
    ]

    return {
        "thm_name": thm_name,
        "var": v,
        "b": b,
        "K": K,
        "raw_code": raw_code,
        "proof_script": proof_script,
        "traces": traces,
    }


def generate_curriculum(num_samples: int = 150, verify_repl: bool = True) -> List[Dict[str, Any]]:
    print("==================================================================")
    print("GENERATING SYNTHETIC ALGEBRAIC RATIONALIZATION CURRICULUM")
    print(f"Target Samples: {num_samples} theorems | REPL Verification: {verify_repl}")
    print("==================================================================")

    random.seed(42)
    repl = LeanREPLClient() if verify_repl else None

    dataset_records = []
    verified_theorems = 0

    combos = []
    for v in VARS:
        for b in B_COEFFS:
            for k in K_CONSTS:
                combos.append((v, b, k))
    random.shuffle(combos)

    combos_to_use = combos[:num_samples]

    for idx, (v, b, k) in enumerate(combos_to_use, 1):
        prob = generate_single_problem(v, b, k, idx)

        # Verify through Lean REPL if requested
        if repl is not None:
            full_code = f"{prob['raw_code'].split(':= by sorry')[0]}:= by\n{prob['proof_script']}"
            ok, lat, err, axioms = repl.verify_full_code(full_code, prob["thm_name"])
            if not ok or "sorryAx" in axioms:
                print(f"  [WARN] Problem {prob['thm_name']} verification failed: {err}")
                continue
            verified_theorems += 1

        # Format traces for ProofBDH training
        for t in prob["traces"]:
            text = f"<hyp> {t['hypotheses']} </hyp> <goal> {t['goal']} <plan> proof </plan><tactic> {t['tactic']}"
            dataset_records.append({
                "theorem": prob["thm_name"],
                "text": text,
                "goal": t["goal"],
                "hypotheses": t["hypotheses"],
                "tactic": t["tactic"],
            })

        if idx % 20 == 0 or idx == len(combos_to_use):
            print(f"  Progress: {idx}/{len(combos_to_use)} theorems processed ({len(dataset_records)} state-action pairs)")

    if repl is not None:
        repl.close()

    # Save to disk
    with open(OUT_FILE, "w", encoding="utf-8") as f:
        for r in dataset_records:
            f.write(json.dumps(r) + "\n")

    print("\n==================================================================")
    print(f"CURRICULUM GENERATION COMPLETE")
    print(f"Verified Theorems: {verified_theorems}/{len(combos_to_use)}")
    print(f"Total State-Action Training Traces: {len(dataset_records)}")
    print(f"Saved to: {OUT_FILE}")
    print("==================================================================")
    return dataset_records


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Generate Rationalization Curriculum")
    parser.add_argument("--num-samples", type=int, default=50, help="Number of synthetic theorems to generate")
    parser.add_argument("--verify-repl", action="store_true", default=True, help="Verify all theorems with Lean REPL")
    args = parser.parse_args()

    generate_curriculum(num_samples=args.num_samples, verify_repl=args.verify_repl)
