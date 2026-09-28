import { describe, it, expect } from 'vitest';
import { toMonacoMarkers, MonacoMarkerSeverity } from './diagnostics';
import type { Diagnostic } from '$lib/ws/generated';

describe('Diagnostics Converter & Model Clamping', () => {
	it('converts 1-based wire diagnostics to Monaco IMarkerData with correct severity', () => {
		const raw: Diagnostic[] = [
			{
				line: 5,
				col: 3,
				end_line: 5,
				end_col: 10,
				severity: 'error',
				message: 'unknown tactic'
			},
			{
				line: 1,
				col: 1,
				end_line: 2,
				end_col: 4,
				severity: 'warning',
				message: 'unused variable'
			},
			{
				line: 2,
				col: 1,
				end_line: 2,
				end_col: 3,
				severity: 'info',
				message: 'tactical note'
			}
		];

		const markers = toMonacoMarkers(raw);
		expect(markers).toHaveLength(3);

		expect(markers[0]?.startLineNumber).toBe(5);
		expect(markers[0]?.startColumn).toBe(3);
		expect(markers[0]?.endLineNumber).toBe(5);
		expect(markers[0]?.endColumn).toBe(10);
		expect(markers[0]?.severity).toBe(MonacoMarkerSeverity.Error);
		expect(markers[0]?.source).toBe('Lean 4');

		expect(markers[1]?.severity).toBe(MonacoMarkerSeverity.Warning);
		expect(markers[2]?.severity).toBe(MonacoMarkerSeverity.Info);
	});

	it('safely clamps lines and columns within the bounds of a model', () => {
		const mockModel = {
			getLineCount: () => 3,
			getLineMaxColumn: (line: number) => {
				if (line === 1) return 12; // "intro n\n"
				if (line === 2) return 8; // "simp\n"
				return 5;
			}
		};

		// Diagnostic points to line 10 (beyond EOF) with column 50
		const raw: Diagnostic[] = [
			{
				line: 10,
				col: 50,
				end_line: 15,
				end_col: 80,
				severity: 'error',
				message: 'stale error beyond EOF'
			},
			{
				line: 0,
				col: 0,
				end_line: 0,
				end_col: 0,
				severity: 'error',
				message: 'zero-based invalid line'
			}
		];

		const markers = toMonacoMarkers(raw, mockModel);
		expect(markers).toHaveLength(2);

		// Line 10 clamped to max line (3), column 50 clamped to max col (5)
		expect(markers[0]?.startLineNumber).toBe(3);
		expect(markers[0]?.startColumn).toBe(5);
		expect(markers[0]?.endLineNumber).toBe(3);
		expect(markers[0]?.endColumn).toBe(5);

		// Line 0 clamped to minimum 1
		expect(markers[1]?.startLineNumber).toBe(1);
		expect(markers[1]?.startColumn).toBe(1);
	});

	it('preserves multi-line diagnostic messages with newlines without crashing', () => {
		const raw: Diagnostic[] = [
			{
				line: 1,
				col: 1,
				end_line: 1,
				end_col: 5,
				severity: 'error',
				message: 'type mismatch\n  expected: ℕ\n  got: String'
			}
		];

		const markers = toMonacoMarkers(raw);
		expect(markers[0]?.message).toBe('type mismatch\n  expected: ℕ\n  got: String');
	});

	it('handles lines containing Unicode surrogate pairs correctly', () => {
		// String with surrogate pair (e.g. math Fraktur 𝔽 U+1D53D has length 2 in UTF-16)
		const lineWithSurrogate = 'have h : 𝔽 := by';
		const mockModel = {
			getLineCount: () => 1,
			getLineMaxColumn: () => lineWithSurrogate.length + 1
		};

		const raw: Diagnostic[] = [
			{
				line: 1,
				col: 10,
				end_line: 1,
				end_col: 14,
				severity: 'error',
				message: 'unknown constant 𝔽'
			}
		];

		const markers = toMonacoMarkers(raw, mockModel);
		expect(markers).toHaveLength(1);
		expect(markers[0]?.startColumn).toBe(10);
		expect(markers[0]?.endColumn).toBe(14);
		expect(markers[0]?.endColumn).toBeLessThanOrEqual(mockModel.getLineMaxColumn());
	});

	it('handles empty diagnostics array cleanly', () => {
		expect(toMonacoMarkers([])).toEqual([]);
	});
});
