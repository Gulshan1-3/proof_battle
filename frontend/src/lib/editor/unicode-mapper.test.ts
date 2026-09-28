import { describe, it, expect } from 'vitest';
import { expandAt } from './unicode-mapper';

describe('Unicode Mapper expandAt pure unit tests', () => {
	it('returns null when no backslash keyword precedes the cursor', () => {
		const model = {
			getLineContent: () => 'intro n'
		};
		const res = expandAt(model, { lineNumber: 1, column: 8 });
		expect(res).toBeNull();
	});

	it('returns null for unknown backslash sequences', () => {
		const model = {
			getLineContent: () => 'have h : \\unknown'
		};
		const res = expandAt(model, { lineNumber: 1, column: 18 });
		expect(res).toBeNull();
	});

	it('correctly maps \\forall to ∀ and computes the exact replacement range', () => {
		const model = {
			getLineContent: () => 'theorem test : \\forall n'
		};
		// Cursor right after \forall (column 23)
		const res = expandAt(model, { lineNumber: 1, column: 23 });
		expect(res).not.toBeNull();
		expect(res!.text).toBe('∀');
		expect(res!.range).toEqual({
			startLineNumber: 1,
			startColumn: 16, // column 23 - length 7 = 16
			endLineNumber: 1,
			endColumn: 23
		});
	});

	it('correctly maps common symbols: exists, mapsto, circ, sum, prod, le, ge, nat', () => {
		const testCases: Array<{ input: string; col: number; expectedText: string; len: number }> = [
			{ input: '\\exists', col: 8, expectedText: '∃', len: 7 },
			{ input: '\\to', col: 4, expectedText: '→', len: 3 },
			{ input: '\\mapsto', col: 8, expectedText: '↦', len: 7 },
			{ input: '\\circ', col: 6, expectedText: '∘', len: 5 },
			{ input: '\\sum', col: 5, expectedText: '∑', len: 4 },
			{ input: '\\prod', col: 6, expectedText: '∏', len: 5 },
			{ input: '\\le', col: 4, expectedText: '≤', len: 3 },
			{ input: '\\ge', col: 4, expectedText: '≥', len: 3 },
			{ input: '\\nat', col: 5, expectedText: 'ℕ', len: 4 },
			{ input: '\\iff', col: 5, expectedText: '↔', len: 4 }
		];

		for (const tc of testCases) {
			const model = { getLineContent: () => tc.input };
			const res = expandAt(model, { lineNumber: 1, column: tc.col });
			expect(res, `Failed on ${tc.input}`).not.toBeNull();
			expect(res!.text).toBe(tc.expectedText);
			expect(res!.range.startColumn).toBe(tc.col - tc.len);
			expect(res!.range.endColumn).toBe(tc.col);
		}
	});

	it('handles mid-line and indented backslash keywords', () => {
		const model = {
			getLineContent: () => '  intro (h : a \\le'
		};
		// Length of '  intro (h : a \le' is 18, so cursor right after \le is col 19
		const res = expandAt(model, { lineNumber: 1, column: 19 });
		expect(res).not.toBeNull();
		expect(res!.text).toBe('≤');
		expect(res!.range.startColumn).toBe(16);
		expect(res!.range.endColumn).toBe(19);
	});
});
