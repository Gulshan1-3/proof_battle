# Learning Lean 4 for ProofBattle

This guide covers the fundamentals of writing tactic proofs in Lean 4. It is written specifically for the problems you encounter in ProofBattle challenges.

---

## 1. How Proofs Work in ProofBattle

In standard Lean 4 development, you define theorems with a signature and a body:

```lean
theorem add_zero (n : Nat) : n + 0 = n := by
  simp
```

In ProofBattle, the server owns the imports and the theorem header. The arena editor only accepts the tactic body that goes after `:= by`.

Your code runs in tactic mode. In tactic mode, you manipulate an active proof state until all goals are closed.

### The Proof State

When Lean elaborates your code, it maintains a goal buffer formatted like this:

```text
n : Nat
h : n > 0
⊢ n + 0 = n
```

- Items above the turnstile (`⊢`) are hypotheses and variables currently in scope.
- The formula below the turnstile (`⊢`) is the goal you need to prove.
- A proof is complete when every branch has zero remaining goals.

---

## 2. Core Tactics Reference

Here are the most frequently used tactics in ProofBattle matches, grouped by their role.

### Starting and Decomposing Goals

#### `intro`
Moves variables and hypotheses from the goal into your local context.

Use it when your goal starts with a universal quantifier (`∀`) or an implication (`→`).

```lean
-- Goal: ∀ a b : Nat, a + b = b + a
intro a b
-- Context now has: a b : Nat
-- Goal: a + b = b + a
```

You can also destructure conjunctions directly in the intro pattern:

```lean
-- Goal: P ∧ Q → Q ∧ P
intro h
-- or destructure immediately:
intro ⟨hP, hQ⟩
```

#### `cases` and `rcases`
Deconstructs a hypothesis that has multiple constructors (such as logical AND `∧`, logical OR `∨`, or an inductive type).

```lean
-- Context has: h : P ∧ Q
cases h with
| intro hP hQ =>
  -- Context now has hP : P and hQ : Q
```

#### `constructor`
Splits a compound goal into subgoals. For example, if the goal is `P ∧ Q`, calling `constructor` produces two separate goals: one for `P` and one for `Q`.

```lean
-- Goal: P ∧ Q
constructor
-- Subgoal 1: ⊢ P
-- Subgoal 2: ⊢ Q
```

### Closing Goals Directly

#### `rfl`
Stands for reflexivity. It closes goals where both sides of an equality reduce to the exact same expression by definition.

```lean
-- Goal: 2 + 2 = 4
rfl
```

#### `exact`
Closes the current goal immediately if you have a term or hypothesis that matches the goal type exactly.

```lean
-- Context has: h : P
-- Goal: ⊢ P
exact h
```

You can also construct anonymous pairs using angle brackets:

```lean
-- Context has: hP : P, hQ : Q
-- Goal: ⊢ Q ∧ P
exact ⟨hQ, hP⟩
```

#### `assumption`
Scans your local context and closes the goal if any hypothesis matches the target.

```lean
-- Context has: h1 : A, h2 : B
-- Goal: ⊢ B
assumption
```

### Transforming Goals and Hypotheses

#### `apply`
Works backward from the goal. If your goal is `Q` and you have a theorem `h : P → Q`, calling `apply h` changes the goal to `P`.

```lean
-- Context has: h : A → B
-- Goal: ⊢ B
apply h
-- New goal: ⊢ A
```

#### `simp`
The equational simplifier. It rewrites expressions using lemmas tagged with the `[simp]` attribute in Mathlib and Lean core.

You can pass specific lemmas to include or exclude:

```lean
simp [Nat.add_comm]
simp [Nat.succ_eq_add_one]
```

#### `rw` (rewrite)
Replaces expressions using an equality hypothesis or theorem from left to right.

```lean
-- Context has: h : a = b
-- Goal: ⊢ a + c = b + c
rw [h]
-- Goal becomes: ⊢ b + c = b + c
```

To rewrite from right to left, prefix the lemma with `←`:

```lean
rw [← h]
```

#### `linarith` and `ring`
Decision procedures for arithmetic:
- `ring` solves polynomial equalities over commutative rings (such as integer or real algebra).
- `linarith` solves linear arithmetic goals and contradictions involving inequalities (`<`, `≤`, `=`, `≥`, `>`).

### Induction

#### `induction`
Performs structural induction on an inductive data type, typically natural numbers (`Nat`) or lists.

```lean
-- Goal: ∀ n : Nat, 0 + n = n
intro n
induction n with
| zero =>
  rfl
| succ n ih =>
  simp [Nat.succ_eq_add_one, ih]
```

---

## 3. Worked Examples

### Example 1: Propositional Logic (Conjunction Commutativity)

Goal:
```lean
∀ P Q : Prop, P ∧ Q → Q ∧ P
```

Solution:
```lean
intro P Q h
exact ⟨h.right, h.left⟩
```

Step-by-step breakdown:
1. `intro P Q h` brings the propositions `P` and `Q` into context, along with assumption `h : P ∧ Q`.
2. `h.left` has type `P` and `h.right` has type `Q`.
3. `⟨h.right, h.left⟩` constructs a proof of `Q ∧ P`.
4. `exact` delivers that term and closes the goal.

Alternatively, using procedural tactics:

```lean
intro P Q ⟨hP, hQ⟩
constructor
· exact hQ
· exact hP
```

---

### Example 2: Natural Number Identity

Goal:
```lean
∀ n : Nat, n + 0 = n
```

Solution:
```lean
intro n
rfl
```

In Lean 4, addition on `Nat` is defined by recursion on the second argument. Because `n + 0` evaluates to `n` definitionally, `rfl` closes the goal in one step.

---

### Example 3: Natural Number Commutativity

Goal:
```lean
∀ a b : Nat, a + b = b + a
```

Solution:
```lean
intro a b
simp [Nat.add_comm]
```

Or using rewrite directly:

```lean
intro a b
rw [Nat.add_comm]
```

---

### Example 4: Proof by Induction

Goal:
```lean
∀ n : Nat, 0 + n = n
```

Unlike `n + 0`, `0 + n` does not compute to `n` definitionally because addition recurses on the second argument. We prove this by induction on `n`:

Solution:
```lean
intro n
induction n with
| zero =>
  rfl
| succ n ih =>
  rw [Nat.add_succ, ih]
```

---

## 4. Understanding Lean Compiler Diagnostics

When your proof does not compile, ProofBattle returns line-accurate diagnostic squiggles in the editor.

### Common Errors and Fixes

| Diagnostic Message | Meaning | Practical Fix |
| :--- | :--- | :--- |
| `unsolved goals` | Your tactic sequence ended, but some branches are still unproven. | Check the proof state in the compiler feedback pane and add missing steps. |
| `unknown identifier 'xyz'` | You referenced a variable or theorem name that is not in scope. | Check your spelling, verify imports, or check if the variable was introduced with `intro`. |
| `type mismatch` | A tactic provided a term of type `A`, but Lean expected type `B`. | Check that hypotheses match the exact direction of equalities or implications. |
| `tactic 'rfl' failed` | The left and right hand sides are not definitionally equal. | Use `simp`, `rw`, or `induction` instead of `rfl`. |
| `no goals to be solved` | You called a tactic after the goal was already closed. | Remove redundant trailing tactics. |

---

## 5. Symbols and Unicode Shortcuts

ProofBattle includes an on-screen symbol bar above the editor. You can also type backslash shortcuts directly in the Monaco editor:

| Shortcut | Symbol | Meaning |
| :--- | :--- | :--- |
| `\to` | `→` | Implication / Function arrow |
| `\and` | `∧` | Logical AND |
| `\or` | `∨` | Logical OR |
| `\not` | `¬` | Negation |
| `\all` | `∀` | Universal quantifier |
| `\ex` | `∃` | Existential quantifier |
| `\<` | `⟨` | Left constructor bracket |
| `\>` | `⟩` | Right constructor bracket |
| `\le` | `≤` | Less than or equal to |
| `\ge` | `≥` | Greater than or equal to |
| `\ne` | `≠` | Not equal |
| `\nat` | `ℕ` | Natural numbers |
| `\Z` | `ℤ` | Integers |
| `\Q` | `ℚ` | Rational numbers |
| `\R` | `ℝ` | Real numbers |

---

## 6. Proving Tips for Competitive Duels

1. Test with Check Only first: Press `Ctrl+Enter` or click "Check Only" to run background elaboration without consuming your submit quota.
2. Watch the submit rate limiter: Submissions are capped at 5 per 60 seconds to prevent brute-force tactic spamming.
3. Never write `sorry`: Any submission containing `sorry` or cheat axioms triggers an immediate server filter rejection.
4. Try `simp` early: Many entry-level theorems in Mathlib reduce to known identities with `simp` or `simp [lemma_name]`.
5. Break compound goals with `constructor`: If you face `A ∧ B`, splitting it into two smaller goals makes progress visible and manageable.
