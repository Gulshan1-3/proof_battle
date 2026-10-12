import type { Problem } from '$lib/ws/generated';

export const MOCK_PROBLEMS: Problem[] = [
	{
		id: 'mock-prob-1',
		goal: '∀ (P Q : Prop), P ∨ Q ↔ Q ∨ P',
		imports: ['import Mathlib.Logic.Basic'],
		difficulty: 2,
		category: 'logic',
		hint: 'Use constructor, then cases on hypothesis',
		duration_ms: 300_000n
	},
	{
		id: 'mock-prob-2',
		goal: '∀ a b : ℕ, a + b = b + a',
		imports: ['import Mathlib.Data.Nat.Basic'],
		difficulty: 2,
		category: 'arithmetic',
		hint: 'Try simp [Nat.add_comm]',
		duration_ms: 300_000n
	},
	{
		id: 'mock-prob-3',
		goal: '∀ P Q : Prop, P ∧ Q → Q ∧ P',
		imports: ['import Mathlib.Logic.Basic'],
		difficulty: 1,
		category: 'logic',
		hint: 'Deconstruct hypothesis with cases, then reconstruct with exact',
		duration_ms: 300_000n
	}
];
