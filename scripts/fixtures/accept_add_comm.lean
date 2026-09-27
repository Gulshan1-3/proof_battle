import Mathlib.Data.Nat.Basic

theorem goal : ∀ a b : ℕ, a + b = b + a := by
  intro a b
  simp [Nat.add_comm]
