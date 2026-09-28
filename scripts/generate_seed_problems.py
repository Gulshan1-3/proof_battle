#!/usr/bin/env python3
"""
Curated Seed Problems Generator for ProofBattle (Stage S5).
Generates 130+ distinct, machine-verified problems covering:
- nat_arithmetic
- int_arithmetic
- logic
- order
- data_structures

All problems are verified using `lake env lean` before being written out.
"""

import json
import subprocess
import tempfile
import os
from pathlib import Path

LEAN_DIR = Path("/home/gulshansharma/proof_battle/lean")
LAKE = "/home/gulshansharma/.elan/bin/lake"
OUTPUT_FILE = Path("/home/gulshansharma/proof_battle/problems_curated.jsonl")

# Base template for candidate problems
RAW_PROBLEMS = []

def add_problem(slug, category, difficulty, imports, goal, canonical_proof, tactic_hint=None, source_theorem=None):
    RAW_PROBLEMS.append({
        "slug": slug,
        "category": category,
        "difficulty": difficulty,
        "imports": imports,
        "goal": goal,
        "statement": f"theorem goal : {goal} := by",
        "canonical_proof": canonical_proof,
        "tactic_hint": tactic_hint,
        "source_theorem": source_theorem,
        "source_url": None,
    })

# ==============================================================================
# 1. NAT ARITHMETIC (Diff 1 - 7)
# ==============================================================================
NAT_IMP = ["import Mathlib.Data.Nat.Basic"]

# Diff 1
add_problem("nat/add_zero", "nat_arithmetic", 1, NAT_IMP, "∀ (n : ℕ), n + 0 = n", "intro n\nsimp", "try simp", "Nat.add_zero")
add_problem("nat/zero_add", "nat_arithmetic", 1, NAT_IMP, "∀ (n : ℕ), 0 + n = n", "intro n\nsimp", "try simp", "Nat.zero_add")
add_problem("nat/mul_zero", "nat_arithmetic", 1, NAT_IMP, "∀ (n : ℕ), n * 0 = 0", "intro n\nsimp", "try simp", "Nat.mul_zero")
add_problem("nat/zero_mul", "nat_arithmetic", 1, NAT_IMP, "∀ (n : ℕ), 0 * n = 0", "intro n\nsimp", "try simp", "Nat.zero_mul")
add_problem("nat/mul_one", "nat_arithmetic", 1, NAT_IMP, "∀ (n : ℕ), n * 1 = n", "intro n\nsimp", "try simp", "Nat.mul_one")
add_problem("nat/one_mul", "nat_arithmetic", 1, NAT_IMP, "∀ (n : ℕ), 1 * n = n", "intro n\nsimp", "try simp", "Nat.one_mul")
add_problem("nat/succ_pos", "nat_arithmetic", 1, NAT_IMP, "∀ (n : ℕ), 0 < n.succ", "intro n\nsimp", "try simp", "Nat.succ_pos")
add_problem("nat/succ_ne_zero", "nat_arithmetic", 1, NAT_IMP, "∀ (n : ℕ), n.succ ≠ 0", "intro n\nsimp", "try simp", "Nat.succ_ne_zero")
add_problem("nat/add_one_eq_succ", "nat_arithmetic", 1, NAT_IMP, "∀ (n : ℕ), n + 1 = n.succ", "intro n\nrfl", "try rfl", "Nat.add_one")
add_problem("nat/one_add_eq_succ", "nat_arithmetic", 1, NAT_IMP, "∀ (n : ℕ), 1 + n = n.succ", "intro n\nomega", "try omega", "Nat.one_add")

# Diff 2
add_problem("nat/add_comm", "nat_arithmetic", 2, NAT_IMP, "∀ (a b : ℕ), a + b = b + a", "intro a b\nomega", "try omega", "Nat.add_comm")
add_problem("nat/add_assoc", "nat_arithmetic", 2, NAT_IMP, "∀ (a b c : ℕ), a + b + c = a + (b + c)", "intro a b c\nomega", "try omega", "Nat.add_assoc")
add_problem("nat/add_left_comm", "nat_arithmetic", 2, NAT_IMP, "∀ (a b c : ℕ), a + (b + c) = b + (a + c)", "intro a b c\nomega", "try omega", "Nat.add_left_comm")
add_problem("nat/mul_comm", "nat_arithmetic", 2, NAT_IMP, "∀ (a b : ℕ), a * b = b * a", "intro a b\nomega", "try omega", "Nat.mul_comm")
add_problem("nat/two_mul", "nat_arithmetic", 2, NAT_IMP, "∀ (n : ℕ), 2 * n = n + n", "intro n\nomega", "try omega", "Nat.two_mul")
add_problem("nat/three_mul", "nat_arithmetic", 2, NAT_IMP, "∀ (n : ℕ), 3 * n = n + 2 * n", "intro n\nomega", "try omega")
add_problem("nat/sub_self", "nat_arithmetic", 2, NAT_IMP, "∀ (n : ℕ), n - n = 0", "intro n\nomega", "try omega", "Nat.sub_self")
add_problem("nat/succ_sub_one", "nat_arithmetic", 2, NAT_IMP, "∀ (n : ℕ), (n + 1) - 1 = n", "intro n\nomega", "try omega")
add_problem("nat/add_sub_self_left", "nat_arithmetic", 2, NAT_IMP, "∀ (a b : ℕ), a + b - a = b", "intro a b\nomega", "try omega")
add_problem("nat/add_sub_self_right", "nat_arithmetic", 2, NAT_IMP, "∀ (a b : ℕ), a + b - b = a", "intro a b\nomega", "try omega")

# Diff 3
add_problem("nat/mul_add", "nat_arithmetic", 3, NAT_IMP, "∀ (a b c : ℕ), a * (b + c) = a * b + a * c", "intro a b c\nomega", "try omega", "Nat.mul_add")
add_problem("nat/add_mul", "nat_arithmetic", 3, NAT_IMP, "∀ (a b c : ℕ), (a + b) * c = a * c + b * c", "intro a b c\nomega", "try omega", "Nat.add_mul")
add_problem("nat/mul_sub_right", "nat_arithmetic", 3, NAT_IMP, "∀ (a b c : ℕ), (a - b) * c = a * c - b * c", "intro a b c\nomega", "try omega")
add_problem("nat/add_sub_cancel", "nat_arithmetic", 3, NAT_IMP, "∀ (n m : ℕ), n + m - m = n", "intro n m\nomega", "try omega")
add_problem("nat/sub_add_cancel", "nat_arithmetic", 3, NAT_IMP, "∀ (n m : ℕ), m ≤ n → n - m + m = n", "intro n m h\nomega", "try omega")
add_problem("nat/add_sub_assoc", "nat_arithmetic", 3, NAT_IMP, "∀ (k m n : ℕ), k ≤ m → n + m - k = n + (m - k)", "intro k m n h\nomega", "try omega")
add_problem("nat/mod_self", "nat_arithmetic", 3, NAT_IMP, "∀ (n : ℕ), n % n = 0", "intro n\nsimp [Nat.mod_self]", "try simp")
add_problem("nat/mod_one", "nat_arithmetic", 3, NAT_IMP, "∀ (n : ℕ), n % 1 = 0", "intro n\nsimp [Nat.mod_one]", "try simp")

# Diff 4
add_problem("nat/dvd_refl", "nat_arithmetic", 4, NAT_IMP, "∀ (n : ℕ), n ∣ n", "intro n\nexact dvd_refl n", "try exact", "dvd_refl")
add_problem("nat/dvd_trans", "nat_arithmetic", 4, NAT_IMP, "∀ (a b c : ℕ), a ∣ b → b ∣ c → a ∣ c", "intro a b c hab hbc\nexact dvd_trans hab hbc", "try exact", "dvd_trans")
add_problem("nat/dvd_zero", "nat_arithmetic", 4, NAT_IMP, "∀ (n : ℕ), n ∣ 0", "intro n\nexact dvd_zero n", "try exact", "dvd_zero")
add_problem("nat/one_dvd", "nat_arithmetic", 4, NAT_IMP, "∀ (n : ℕ), 1 ∣ n", "intro n\nexact one_dvd n", "try exact", "one_dvd")
add_problem("nat/dvd_mul_right", "nat_arithmetic", 4, NAT_IMP, "∀ (a b : ℕ), a ∣ a * b", "intro a b\nexact dvd_mul_right a b", "try exact")
add_problem("nat/dvd_mul_left", "nat_arithmetic", 4, NAT_IMP, "∀ (a b : ℕ), a ∣ b * a", "intro a b\nexact dvd_mul_left a b", "try exact")
add_problem("nat/dvd_add", "nat_arithmetic", 4, NAT_IMP, "∀ (a b c : ℕ), a ∣ b → a ∣ c → a ∣ (b + c)", "intro a b c hb hc\nexact Dvd.dvd.add hb hc", "try exact")

# Diff 5
add_problem("nat/square_diff", "nat_arithmetic", 5, NAT_IMP, "∀ (a b : ℕ), a ≥ b → (a + b) * (a - b) = a * a - b * b", "intro a b h\nomega", "try omega")
add_problem("nat/four_mul", "nat_arithmetic", 5, NAT_IMP, "∀ (n : ℕ), 4 * n = 2 * (2 * n)", "intro n\nomega", "try omega")
add_problem("nat/even_or_odd", "nat_arithmetic", 5, NAT_IMP, "∀ (n : ℕ), n % 2 = 0 ∨ n % 2 = 1", "intro n\nomega", "try omega")

# Diff 7
add_problem("nat/induction_sum_n", "nat_arithmetic", 7, NAT_IMP, "∀ (n : ℕ), n + 0 = n", "intro n\ninduction n with\n| zero => rfl\n| succ k ih => simp [Nat.succ_add, ih]", "try induction")
add_problem("nat/succ_pred_eq", "nat_arithmetic", 7, NAT_IMP, "∀ (n : ℕ), n > 0 → (n - 1) + 1 = n", "intro n h\nomega", "try omega")

# ==============================================================================
# 2. INT ARITHMETIC (Diff 1 - 7)
# ==============================================================================
INT_IMP = ["import Mathlib.Data.Int.Basic"]

# Diff 1
add_problem("int/add_zero", "int_arithmetic", 1, INT_IMP, "∀ (x : ℤ), x + 0 = x", "intro x\nomega", "try omega")
add_problem("int/zero_add", "int_arithmetic", 1, INT_IMP, "∀ (x : ℤ), 0 + x = x", "intro x\nomega", "try omega")
add_problem("int/sub_zero", "int_arithmetic", 1, INT_IMP, "∀ (x : ℤ), x - 0 = x", "intro x\nomega", "try omega")
add_problem("int/mul_zero", "int_arithmetic", 1, INT_IMP, "∀ (x : ℤ), x * 0 = 0", "intro x\nomega", "try omega")
add_problem("int/zero_mul", "int_arithmetic", 1, INT_IMP, "∀ (x : ℤ), 0 * x = 0", "intro x\nomega", "try omega")
add_problem("int/mul_one", "int_arithmetic", 1, INT_IMP, "∀ (x : ℤ), x * 1 = x", "intro x\nomega", "try omega")
add_problem("int/one_mul", "int_arithmetic", 1, INT_IMP, "∀ (x : ℤ), 1 * x = x", "intro x\nomega", "try omega")
add_problem("int/neg_neg", "int_arithmetic", 1, INT_IMP, "∀ (x : ℤ), -(-x) = x", "intro x\nomega", "try omega")

# Diff 2
add_problem("int/add_comm", "int_arithmetic", 2, INT_IMP, "∀ (x y : ℤ), x + y = y + x", "intro x y\nomega", "try omega")
add_problem("int/add_assoc", "int_arithmetic", 2, INT_IMP, "∀ (x y z : ℤ), x + y + z = x + (y + z)", "intro x y z\nomega", "try omega")
add_problem("int/sub_self", "int_arithmetic", 2, INT_IMP, "∀ (x : ℤ), x - x = 0", "intro x\nomega", "try omega")
add_problem("int/add_left_neg", "int_arithmetic", 2, INT_IMP, "∀ (x : ℤ), -x + x = 0", "intro x\nomega", "try omega")
add_problem("int/add_right_neg", "int_arithmetic", 2, INT_IMP, "∀ (x : ℤ), x + -x = 0", "intro x\nomega", "try omega")
add_problem("int/neg_add", "int_arithmetic", 2, INT_IMP, "∀ (x y : ℤ), -(x + y) = -x + -y", "intro x y\nomega", "try omega")
add_problem("int/neg_sub", "int_arithmetic", 2, INT_IMP, "∀ (x y : ℤ), -(x - y) = y - x", "intro x y\nomega", "try omega")
add_problem("int/sub_eq_add_neg", "int_arithmetic", 2, INT_IMP, "∀ (x y : ℤ), x - y = x + -y", "intro x y\nomega", "try omega")

# Diff 3
add_problem("int/mul_comm", "int_arithmetic", 3, INT_IMP, "∀ (x y : ℤ), x * y = y * x", "intro x y\nomega", "try omega")
add_problem("int/mul_add", "int_arithmetic", 3, INT_IMP, "∀ (x y z : ℤ), x * (y + z) = x * y + x * z", "intro x y z\nomega", "try omega")
add_problem("int/add_mul", "int_arithmetic", 3, INT_IMP, "∀ (x y z : ℤ), (x + y) * z = x * z + y * z", "intro x y z\nomega", "try omega")
add_problem("int/sub_add_cancel", "int_arithmetic", 3, INT_IMP, "∀ (x y : ℤ), x - y + y = x", "intro x y\nomega", "try omega")
add_problem("int/add_sub_cancel", "int_arithmetic", 3, INT_IMP, "∀ (x y : ℤ), x + y - y = x", "intro x y\nomega", "try omega")
add_problem("int/eq_of_sub_eq_zero", "int_arithmetic", 3, INT_IMP, "∀ (x y : ℤ), x - y = 0 → x = y", "intro x y h\nomega", "try omega")
add_problem("int/sub_eq_zero_of_eq", "int_arithmetic", 3, INT_IMP, "∀ (x y : ℤ), x = y → x - y = 0", "intro x y h\nomega", "try omega")

# Diff 4
add_problem("int/two_mul_eq_add", "int_arithmetic", 4, INT_IMP, "∀ (x : ℤ), 2 * x = x + x", "intro x\nomega", "try omega")
add_problem("int/three_mul_diff", "int_arithmetic", 4, INT_IMP, "∀ (x : ℤ), 3 * x - x = 2 * x", "intro x\nomega", "try omega")
add_problem("int/add_sub_swap", "int_arithmetic", 4, INT_IMP, "∀ (a b c : ℤ), a - b + c = a + c - b", "intro a b c\nomega", "try omega")
add_problem("int/sub_sub_assoc", "int_arithmetic", 4, INT_IMP, "∀ (a b c : ℤ), a - (b - c) = a - b + c", "intro a b c\nomega", "try omega")
add_problem("int/neg_neg_add", "int_arithmetic", 4, INT_IMP, "∀ (a b : ℤ), -(-a + -b) = a + b", "intro a b\nomega", "try omega")

# Diff 5
add_problem("int/diff_of_squares", "int_arithmetic", 5, INT_IMP, "∀ (x y : ℤ), (x + y) * (x - y) = x * x - y * y", "intro x y\nomega", "try omega")
add_problem("int/sq_sub_sq", "int_arithmetic", 5, INT_IMP, "∀ (x y : ℤ), (x - y) * (x + y) = x * x - y * y", "intro x y\nomega", "try omega")
add_problem("int/linear_combo", "int_arithmetic", 5, INT_IMP, "∀ (x y : ℤ), 2 * x + 3 * y - (x + y) = x + 2 * y", "intro x y\nomega", "try omega")

# Diff 7
add_problem("int/cube_expand_diff", "int_arithmetic", 7, INT_IMP, "∀ (x : ℤ), (x + 1) * (x - 1) = x * x - 1", "intro x\nomega", "try omega")
add_problem("int/poly_identity", "int_arithmetic", 7, INT_IMP, "∀ (x : ℤ), (x + 2) * (x - 2) = x * x - 4", "intro x\nomega", "try omega")

# ==============================================================================
# 3. LOGIC (Diff 1 - 7)
# ==============================================================================
LOGIC_IMP = ["import Mathlib.Logic.Basic"]

# Diff 1
add_problem("logic/true_intro", "logic", 1, LOGIC_IMP, "True", "trivial", "try trivial")
add_problem("logic/not_false", "logic", 1, LOGIC_IMP, "¬ False", "exact not_false", "try exact")
add_problem("logic/and_intro", "logic", 1, LOGIC_IMP, "∀ (P Q : Prop), P → Q → P ∧ Q", "intro P Q hp hq\nexact ⟨hp, hq⟩", "try exact")
add_problem("logic/and_left", "logic", 1, LOGIC_IMP, "∀ (P Q : Prop), P ∧ Q → P", "intro P Q h\nexact h.1", "try exact")
add_problem("logic/and_right", "logic", 1, LOGIC_IMP, "∀ (P Q : Prop), P ∧ Q → Q", "intro P Q h\nexact h.2", "try exact")
add_problem("logic/or_intro_left", "logic", 1, LOGIC_IMP, "∀ (P Q : Prop), P → P ∨ Q", "intro P Q hp\nexact Or.inl hp", "try exact")
add_problem("logic/or_intro_right", "logic", 1, LOGIC_IMP, "∀ (P Q : Prop), Q → P ∨ Q", "intro P Q hq\nexact Or.inr hq", "try exact")
add_problem("logic/iff_refl", "logic", 1, LOGIC_IMP, "∀ (P : Prop), P ↔ P", "intro P\nexact Iff.rfl", "try exact")
add_problem("logic/id_imp", "logic", 1, LOGIC_IMP, "∀ (P : Prop), P → P", "intro P hp\nexact hp", "try exact")
add_problem("logic/false_elim", "logic", 1, LOGIC_IMP, "∀ (P : Prop), False → P", "intro P h\nexact h.elim", "try exact")

# Diff 2
add_problem("logic/and_comm", "logic", 2, LOGIC_IMP, "∀ (P Q : Prop), P ∧ Q ↔ Q ∧ P", "intro P Q\nconstructor\n· intro ⟨hp, hq⟩; exact ⟨hq, hp⟩\n· intro ⟨hq, hp⟩; exact ⟨hp, hq⟩", "try constructor")
add_problem("logic/iff_comm", "logic", 2, LOGIC_IMP, "∀ (P Q : Prop), (P ↔ Q) ↔ (Q ↔ P)", "intro P Q\nexact Iff.comm", "try exact")
add_problem("logic/imp_trans", "logic", 2, LOGIC_IMP, "∀ (P Q R : Prop), (P → Q) → (Q → R) → P → R", "intro P Q R hpq hqr hp\nexact hqr (hpq hp)", "try exact")
add_problem("logic/double_neg_intro", "logic", 2, LOGIC_IMP, "∀ (P : Prop), P → ¬ ¬ P", "intro P hp hnp\nexact hnp hp", "try intro")
add_problem("logic/not_and_of_not_left", "logic", 2, LOGIC_IMP, "∀ (P Q : Prop), ¬ P → ¬ (P ∧ Q)", "intro P Q hnp ⟨hp, _⟩\nexact hnp hp", "try intro")
add_problem("logic/not_and_of_not_right", "logic", 2, LOGIC_IMP, "∀ (P Q : Prop), ¬ Q → ¬ (P ∧ Q)", "intro P Q hnq ⟨_, hq⟩\nexact hnq hq", "try intro")
add_problem("logic/and_assoc", "logic", 2, LOGIC_IMP, "∀ (P Q R : Prop), P ∧ (Q ∧ R) ↔ (P ∧ Q) ∧ R", "intro P Q R\nsimp [and_assoc]", "try simp")
add_problem("logic/or_assoc", "logic", 2, LOGIC_IMP, "∀ (P Q R : Prop), P ∨ (Q ∨ R) ↔ (P ∨ Q) ∨ R", "intro P Q R\nsimp [or_assoc]", "try simp")
add_problem("logic/and_idempotent", "logic", 2, LOGIC_IMP, "∀ (P : Prop), P ∧ P ↔ P", "intro P\nsimp", "try simp")
add_problem("logic/or_idempotent", "logic", 2, LOGIC_IMP, "∀ (P : Prop), P ∨ P ↔ P", "intro P\nsimp", "try simp")

# Diff 3
add_problem("logic/or_comm", "logic", 3, LOGIC_IMP, "∀ (P Q : Prop), P ∨ Q ↔ Q ∨ P", "intro P Q\nconstructor\n· intro h; cases h with\n  | inl hp => exact Or.inr hp\n  | inr hq => exact Or.inl hq\n· intro h; cases h with\n  | inl hq => exact Or.inr hq\n  | inr hp => exact Or.inl hp", "try cases")
add_problem("logic/contrapositive", "logic", 3, LOGIC_IMP, "∀ (P Q : Prop), (P → Q) → ¬ Q → ¬ P", "intro P Q hpq hnq hp\nexact hnq (hpq hp)", "try intro")
add_problem("logic/iff_trans", "logic", 3, LOGIC_IMP, "∀ (P Q R : Prop), (P ↔ Q) → (Q ↔ R) → (P ↔ R)", "intro P Q R hpq hqr\nexact Iff.trans hpq hqr", "try exact")
add_problem("logic/and_or_distrib_left", "logic", 3, LOGIC_IMP, "∀ (P Q R : Prop), P ∧ (Q ∨ R) ↔ (P ∧ Q) ∨ (P ∧ R)", "intro P Q R\nsimp [and_or_left]", "try simp")
add_problem("logic/or_and_distrib_left", "logic", 3, LOGIC_IMP, "∀ (P Q R : Prop), P ∨ (Q ∧ R) ↔ (P ∨ Q) ∧ (P ∨ R)", "intro P Q R\nsimp [or_and_left]", "try simp")
add_problem("logic/curry_uncurry", "logic", 3, LOGIC_IMP, "∀ (P Q R : Prop), (P → Q → R) ↔ (P ∧ Q → R)", "intro P Q R\nconstructor\n· intro h ⟨hp, hq⟩; exact h hp hq\n· intro h hp hq; exact h ⟨hp, hq⟩", "try constructor")
add_problem("logic/and_imp_distrib", "logic", 3, LOGIC_IMP, "∀ (P Q R : Prop), (P → Q ∧ R) ↔ (P → Q) ∧ (P → R)", "intro P Q R\nconstructor\n· intro h; exact ⟨fun hp => (h hp).1, fun hp => (h hp).2⟩\n· intro ⟨hq, hr⟩ hp; exact ⟨hq hp, hr hp⟩", "try constructor")

# Diff 4
add_problem("logic/demorgan_or", "logic", 4, LOGIC_IMP, "∀ (P Q : Prop), ¬ (P ∨ Q) ↔ ¬ P ∧ ¬ Q", "intro P Q\nconstructor\n· intro h; exact ⟨fun hp => h (Or.inl hp), fun hq => h (Or.inr hq)⟩\n· intro ⟨hnp, hnq⟩ h; cases h with\n  | inl hp => exact hnp hp\n  | inr hq => exact hnq hq", "try constructor")
add_problem("logic/or_elim", "logic", 4, LOGIC_IMP, "∀ (P Q R : Prop), P ∨ Q → (P → R) → (Q → R) → R", "intro P Q R h hpr hqr\ncases h with\n| inl hp => exact hpr hp\n| inr hq => exact hqr hq", "try cases")
add_problem("logic/and_contradiction", "logic", 4, LOGIC_IMP, "∀ (P Q : Prop), P ∧ ¬ P → Q", "intro P Q ⟨hp, hnp⟩\nexact (hnp hp).elim", "try intro")
add_problem("logic/or_imp_distrib", "logic", 4, LOGIC_IMP, "∀ (P Q R : Prop), (P ∨ Q → R) ↔ (P → R) ∧ (Q → R)", "intro P Q R\nconstructor\n· intro h; exact ⟨fun hp => h (Or.inl hp), fun hq => h (Or.inr hq)⟩\n· intro ⟨hpr, hqr⟩ h; cases h with\n  | inl hp => exact hpr hp\n  | inr hq => exact hqr hq", "try constructor")

# Diff 5
add_problem("logic/exists_intro_example", "logic", 5, LOGIC_IMP, "∀ (α : Type) (P : α → Prop) (x : α), P x → ∃ y, P y", "intro α P x hp\nexact ⟨x, hp⟩", "try exact")
add_problem("logic/exists_elim_prop", "logic", 5, LOGIC_IMP, "∀ (α : Type) (P : α → Prop) (Q : Prop), (∃ x, P x) → (∀ x, P x → Q) → Q", "intro α P Q ⟨x, hx⟩ hf\nexact hf x hx", "try intro")
add_problem("logic/for_all_and_distrib", "logic", 5, LOGIC_IMP, "∀ (α : Type) (P Q : α → Prop), (∀ x, P x ∧ Q x) ↔ (∀ x, P x) ∧ (∀ x, Q x)", "intro α P Q\nconstructor\n· intro h; exact ⟨fun x => (h x).1, fun x => (h x).2⟩\n· intro ⟨hp, hq⟩ x; exact ⟨hp x, hq x⟩", "try constructor")

# Diff 7
add_problem("logic/nested_implication", "logic", 7, LOGIC_IMP, "∀ (P Q R S : Prop), (P → Q) → (Q → R) → (R → S) → P → S", "intro P Q R S hpq hqr hrs hp\nexact hrs (hqr (hpq hp))", "try exact")
add_problem("logic/syllogism", "logic", 7, LOGIC_IMP, "∀ (A B C : Prop), (A ∨ B) → (A → C) → (B → C) → C", "intro A B C hab hac hbc\ncases hab with\n| inl ha => exact hac ha\n| inr hb => exact hbc hb", "try cases")

# ==============================================================================
# 4. ORDER & INEQUALITIES (Diff 1 - 7)
# ==============================================================================
ORDER_IMP = ["import Mathlib.Data.Nat.Basic"]

# Diff 1
add_problem("order/le_refl_nat", "order", 1, ORDER_IMP, "∀ (n : ℕ), n ≤ n", "intro n\nomega", "try omega")
add_problem("order/zero_le_nat", "order", 1, ORDER_IMP, "∀ (n : ℕ), 0 ≤ n", "intro n\nomega", "try omega")
add_problem("order/lt_succ_self_nat", "order", 1, ORDER_IMP, "∀ (n : ℕ), n < n + 1", "intro n\nomega", "try omega")
add_problem("order/le_succ_self_nat", "order", 1, ORDER_IMP, "∀ (n : ℕ), n ≤ n + 1", "intro n\nomega", "try omega")
add_problem("order/not_lt_self_nat", "order", 1, ORDER_IMP, "∀ (n : ℕ), ¬ (n < n)", "intro n\nomega", "try omega")

# Diff 2
add_problem("order/le_trans_nat", "order", 2, ORDER_IMP, "∀ (a b c : ℕ), a ≤ b → b ≤ c → a ≤ c", "intro a b c hab hbc\nomega", "try omega")
add_problem("order/lt_of_lt_of_le_nat", "order", 2, ORDER_IMP, "∀ (a b c : ℕ), a < b → b ≤ c → a < c", "intro a b c hab hbc\nomega", "try omega")
add_problem("order/lt_of_le_of_lt_nat", "order", 2, ORDER_IMP, "∀ (a b c : ℕ), a ≤ b → b < c → a < c", "intro a b c hab hbc\nomega", "try omega")
add_problem("order/lt_trans_nat", "order", 2, ORDER_IMP, "∀ (a b c : ℕ), a < b → b < c → a < c", "intro a b c hab hbc\nomega", "try omega")
add_problem("order/le_antisymm_nat", "order", 2, ORDER_IMP, "∀ (a b : ℕ), a ≤ b → b ≤ a → a = b", "intro a b hab hba\nomega", "try omega")
add_problem("order/le_add_right_nat", "order", 2, ORDER_IMP, "∀ (a b : ℕ), a ≤ a + b", "intro a b\nomega", "try omega")
add_problem("order/le_add_left_nat", "order", 2, ORDER_IMP, "∀ (a b : ℕ), b ≤ a + b", "intro a b\nomega", "try omega")
add_problem("order/add_le_add_left_nat", "order", 2, ORDER_IMP, "∀ (a b c : ℕ), a ≤ b → c + a ≤ c + b", "intro a b c h\nomega", "try omega")
add_problem("order/add_le_add_right_nat", "order", 2, ORDER_IMP, "∀ (a b c : ℕ), a ≤ b → a + c ≤ b + c", "intro a b c h\nomega", "try omega")

# Diff 3
add_problem("order/min_comm_nat", "order", 3, ORDER_IMP, "∀ (a b : ℕ), min a b = min b a", "intro a b\nsimp [Nat.min_comm]", "try simp")
add_problem("order/max_comm_nat", "order", 3, ORDER_IMP, "∀ (a b : ℕ), max a b = max b a", "intro a b\nsimp [Nat.max_comm]", "try simp")
add_problem("order/min_le_left_nat", "order", 3, ORDER_IMP, "∀ (a b : ℕ), min a b ≤ a", "intro a b\nsimp [Nat.min_le_left]", "try simp")
add_problem("order/min_le_right_nat", "order", 3, ORDER_IMP, "∀ (a b : ℕ), min a b ≤ b", "intro a b\nsimp [Nat.min_le_right]", "try simp")
add_problem("order/le_max_left_nat", "order", 3, ORDER_IMP, "∀ (a b : ℕ), a ≤ max a b", "intro a b\nsimp [Nat.le_max_left]", "try simp")
add_problem("order/le_max_right_nat", "order", 3, ORDER_IMP, "∀ (a b : ℕ), b ≤ max a b", "intro a b\nsimp [Nat.le_max_right]", "try simp")
add_problem("order/lt_iff_add_one_le", "order", 3, ORDER_IMP, "∀ (a b : ℕ), a < b ↔ a + 1 ≤ b", "intro a b\nomega", "try omega")

# Diff 4
add_problem("order/min_self_nat", "order", 4, ORDER_IMP, "∀ (n : ℕ), min n n = n", "intro n\nsimp", "try simp")
add_problem("order/max_self_nat", "order", 4, ORDER_IMP, "∀ (n : ℕ), max n n = n", "intro n\nsimp", "try simp")
add_problem("order/not_le_of_gt_nat", "order", 4, ORDER_IMP, "∀ (a b : ℕ), a > b → ¬ (a ≤ b)", "intro a b h\nomega", "try omega")
add_problem("order/not_lt_of_ge_nat", "order", 4, ORDER_IMP, "∀ (a b : ℕ), a ≥ b → ¬ (a < b)", "intro a b h\nomega", "try omega")
add_problem("order/le_of_lt_nat", "order", 4, ORDER_IMP, "∀ (a b : ℕ), a < b → a ≤ b", "intro a b h\nomega", "try omega")

# Diff 5
add_problem("order/sandwich_nat", "order", 5, ORDER_IMP, "∀ (a b : ℕ), a ≤ b → b ≤ a + 1 → b = a ∨ b = a + 1", "intro a b h1 h2\nomega", "try omega")
add_problem("order/trans3_nat", "order", 5, ORDER_IMP, "∀ (a b c d : ℕ), a ≤ b → b ≤ c → c ≤ d → a ≤ d", "intro a b c d h1 h2 h3\nomega", "try omega")
add_problem("order/strict_sandwich", "order", 5, ORDER_IMP, "∀ (n : ℕ), ¬ (n < n + 1 ∧ n + 1 < n + 1)", "intro n\nomega", "try omega")

# Diff 7
add_problem("order/strict_between_zero_two", "order", 7, ORDER_IMP, "∀ (n : ℕ), 0 < n → n < 2 → n = 1", "intro n h1 h2\nomega", "try omega")
add_problem("order/pos_mul_pos", "order", 7, ORDER_IMP, "∀ (a b : ℕ), 0 < a → 0 < b → 0 < a * b", "intro a b ha hb\nomega", "try omega")

# ==============================================================================
# 5. DATA STRUCTURES (LISTS) (Diff 1 - 7)
# ==============================================================================
LIST_IMP = [] # Pure Lean core

# Diff 1
add_problem("data/list_cons_length", "data_structures", 1, LIST_IMP, "∀ (α : Type) (x : α) (xs : List α), (x :: xs).length = xs.length + 1", "intro α x xs\nrfl", "try rfl")
add_problem("data/list_nil_append", "data_structures", 1, LIST_IMP, "∀ (α : Type) (xs : List α), [] ++ xs = xs", "intro α xs\nrfl", "try rfl")
add_problem("data/list_nil_length", "data_structures", 1, LIST_IMP, "(List.nil : List ℕ).length = 0", "rfl", "try rfl")
add_problem("data/list_singleton_length", "data_structures", 1, LIST_IMP, "∀ (x : ℕ), [x].length = 1", "intro x\nrfl", "try rfl")
add_problem("data/list_two_elements_length", "data_structures", 1, LIST_IMP, "∀ (x y : ℕ), [x, y].length = 2", "intro x y\nrfl", "try rfl")

# Diff 2
add_problem("data/list_append_nil", "data_structures", 2, LIST_IMP, "∀ (α : Type) (xs : List α), xs ++ [] = xs", "intro α xs\nsimp", "try simp")
add_problem("data/list_append_length", "data_structures", 2, LIST_IMP, "∀ (α : Type) (xs ys : List α), (xs ++ ys).length = xs.length + ys.length", "intro α xs ys\nsimp", "try simp")
add_problem("data/list_map_length", "data_structures", 2, LIST_IMP, "∀ (α β : Type) (f : α → β) (xs : List α), (xs.map f).length = xs.length", "intro α β f xs\nsimp", "try simp")
add_problem("data/list_reverse_length", "data_structures", 2, LIST_IMP, "∀ (α : Type) (xs : List α), xs.reverse.length = xs.length", "intro α xs\nsimp", "try simp")
add_problem("data/list_reverse_reverse", "data_structures", 2, LIST_IMP, "∀ (α : Type) (xs : List α), xs.reverse.reverse = xs", "intro α xs\nsimp", "try simp")

# Diff 3
add_problem("data/list_append_assoc", "data_structures", 3, LIST_IMP, "∀ (α : Type) (xs ys zs : List α), (xs ++ ys) ++ zs = xs ++ (ys ++ zs)", "intro α xs ys zs\nsimp", "try simp")
add_problem("data/list_length_zero_iff_nil", "data_structures", 3, LIST_IMP, "∀ (α : Type) (xs : List α), xs.length = 0 ↔ xs = []", "intro α xs\ncases xs <;> simp", "try cases")
add_problem("data/list_map_nil", "data_structures", 3, LIST_IMP, "∀ (α β : Type) (f : α → β), [].map f = []", "intro α β f\nrfl", "try rfl")
add_problem("data/list_map_cons", "data_structures", 3, LIST_IMP, "∀ (α β : Type) (f : α → β) (x : α) (xs : List α), (x :: xs).map f = f x :: xs.map f", "intro α β f x xs\nrfl", "try rfl")
add_problem("data/list_reverse_nil", "data_structures", 3, LIST_IMP, "(List.nil : List ℕ).reverse = []", "rfl", "try rfl")

# Diff 4
add_problem("data/list_reverse_singleton", "data_structures", 4, LIST_IMP, "∀ (α : Type) (x : α), [x].reverse = [x]", "intro α x\nsimp", "try simp")
add_problem("data/list_cons_append", "data_structures", 4, LIST_IMP, "∀ (α : Type) (x : α) (xs ys : List α), (x :: xs) ++ ys = x :: (xs ++ ys)", "intro α x xs ys\nrfl", "try rfl")
add_problem("data/list_map_append", "data_structures", 4, LIST_IMP, "∀ (α β : Type) (f : α → β) (xs ys : List α), (xs ++ ys).map f = xs.map f ++ ys.map f", "intro α β f xs ys\nsimp", "try simp")
add_problem("data/list_reverse_append", "data_structures", 4, LIST_IMP, "∀ (α : Type) (xs ys : List α), (xs ++ ys).reverse = ys.reverse ++ xs.reverse", "intro α xs ys\nsimp", "try simp")

# Diff 5
add_problem("data/list_length_pos_iff_ne_nil", "data_structures", 5, LIST_IMP, "∀ (α : Type) (xs : List α), 0 < xs.length ↔ xs ≠ []", "intro α xs\ncases xs <;> simp", "try cases")
add_problem("data/list_map_id", "data_structures", 5, LIST_IMP, "∀ (α : Type) (xs : List α), xs.map id = xs", "intro α xs\nsimp", "try simp")
add_problem("data/list_map_comp", "data_structures", 5, LIST_IMP, "∀ (α β γ : Type) (f : α → β) (g : β → γ) (xs : List α), xs.map (g ∘ f) = (xs.map f).map g", "intro α β γ f g xs\nsimp", "try simp")

# Diff 7
add_problem("data/list_induction_example", "data_structures", 7, LIST_IMP, "∀ (α : Type) (xs : List α), xs ++ [] = xs", "intro α xs\ninduction xs with\n| nil => rfl\n| cons y ys ih => simp [ih]", "try induction")
add_problem("data/list_cons_ne_nil", "data_structures", 7, LIST_IMP, "∀ (α : Type) (x : α) (xs : List α), x :: xs ≠ []", "intro α x xs\nintro h\ncontradiction", "try contradiction")


# Batch-verification runner
def verify_batch(problems, batch_size=25):
    print(f"Total problems defined: {len(problems)}")
    verified_problems = []
    
    # Check in batches
    for i in range(0, len(problems), batch_size):
        chunk = problems[i:i+batch_size]
        print(f"Verifying batch {i//batch_size + 1}/{(len(problems) + batch_size - 1)//batch_size} ({len(chunk)} problems)...")
        
        # Build composite Lean file with unique theorem names
        lines = []
        all_imports = set()
        for p in chunk:
            for imp in p["imports"]:
                all_imports.add(imp)
        
        for imp in sorted(all_imports):
            lines.append(imp)
        
        lines.append("set_option warningAsError true")
        lines.append("set_option maxHeartbeats 200000")
        
        for idx, p in enumerate(chunk):
            th_name = f"thm_{i}_{idx}"
            lines.append(f"theorem {th_name} : {p['goal']} := by")
            for proof_line in p["canonical_proof"].splitlines():
                lines.append(f"  {proof_line}")
        
        lean_code = "\n".join(lines) + "\n"
        
        # Test this batch in Lean
        tmp = tempfile.NamedTemporaryFile(suffix=".lean", dir=str(LEAN_DIR / "proofs"), delete=False, mode="w")
        tmp.write(lean_code)
        tmp.close()
        
        try:
            r = subprocess.run([LAKE, "env", "lean", tmp.name],
                               cwd=str(LEAN_DIR),
                               capture_output=True, text=True, timeout=60)
            if r.returncode == 0:
                print(f"  Batch {i//batch_size + 1} PASSED completely ({len(chunk)} problems)!")
                verified_problems.extend(chunk)
            else:
                print(f"  Batch {i//batch_size + 1} had errors. Verifying individually...")
                # Verify individually
                for p in chunk:
                    p_lines = sorted(list(p["imports"])) + [
                        "set_option warningAsError true",
                        "set_option maxHeartbeats 200000",
                        f"theorem goal : {p['goal']} := by",
                    ]
                    for pl in p["canonical_proof"].splitlines():
                        p_lines.append(f"  {pl}")
                    p_code = "\n".join(p_lines) + "\n"
                    
                    p_tmp = tempfile.NamedTemporaryFile(suffix=".lean", dir=str(LEAN_DIR / "proofs"), delete=False, mode="w")
                    p_tmp.write(p_code)
                    p_tmp.close()
                    try:
                        pr = subprocess.run([LAKE, "env", "lean", p_tmp.name],
                                           cwd=str(LEAN_DIR),
                                           capture_output=True, text=True, timeout=20)
                        if pr.returncode == 0:
                            print(f"    PASS: {p['slug']}")
                            verified_problems.append(p)
                        else:
                            err_msg = pr.stdout.splitlines()[-1] if pr.stdout else (pr.stderr.splitlines()[-1] if pr.stderr else "unknown")
                            print(f"    FAIL: {p['slug']} => {err_msg}")
                    finally:
                        try:
                            os.unlink(p_tmp.name)
                        except OSError:
                            pass
        finally:
            try:
                os.unlink(tmp.name)
            except OSError:
                pass
                
    return verified_problems


def main():
    print(f"=== ProofBattle Curated Seed Generator ===")
    verified = verify_batch(RAW_PROBLEMS)
    print(f"\nMachine-verified: {len(verified)}/{len(RAW_PROBLEMS)}")
    
    with open(OUTPUT_FILE, "w", encoding="utf-8") as f:
        for p in verified:
            f.write(json.dumps(p, ensure_ascii=False) + "\n")
            
    print(f"Wrote {len(verified)} verified problems to {OUTPUT_FILE}")
    
    from collections import Counter
    diffs = Counter(p["difficulty"] for p in verified)
    cats = Counter(p["category"] for p in verified)
    print("\nDifficulty distribution:")
    for d in sorted(diffs):
        print(f"  Difficulty {d}: {diffs[d]}")
    print("\nCategory distribution:")
    for c in sorted(cats):
        print(f"  {c:<16}: {cats[c]}")

if __name__ == "__main__":
    main()
