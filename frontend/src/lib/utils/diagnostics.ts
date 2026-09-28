import type { Diagnostic } from '$lib/ws/generated';
import type { editor } from 'monaco-editor';

// MarkerSeverity enum values from monaco-editor:
// Hint = 1, Info = 2, Warning = 4, Error = 8
export const MonacoMarkerSeverity = {
	Hint: 1,
	Info: 2,
	Warning: 4,
	Error: 8
} as const;

export interface MinimalTextModel {
	getLineCount(): number;
	getLineMaxColumn(lineNumber: number): number;
}

/**
 * Converts wire protocol Diagnostic objects to Monaco IMarkerData.
 * Safely clamps lines and columns into the model range so out-of-range
 * diagnostics (e.g. following line deletions or beyond EOF) do not throw or crash.
 */
export function toMonacoMarkers(
	diagnostics: Diagnostic[],
	model?: MinimalTextModel | null
): editor.IMarkerData[] {
	if (!diagnostics || diagnostics.length === 0) {
		return [];
	}

	const maxLine = model ? Math.max(1, model.getLineCount()) : Infinity;

	return diagnostics.map((d) => {
		// Wire protocol provides 1-based lines and columns
		const rawStartLine = Math.max(1, d.line || 1);
		const startLineNumber = model ? Math.min(maxLine, rawStartLine) : rawStartLine;

		const maxStartCol = model ? model.getLineMaxColumn(startLineNumber) : Infinity;
		const rawStartCol = Math.max(1, d.col || 1);
		const startColumn = model ? Math.min(maxStartCol, rawStartCol) : rawStartCol;

		const rawEndLine = Math.max(startLineNumber, d.end_line || startLineNumber);
		const endLineNumber = model ? Math.min(maxLine, rawEndLine) : rawEndLine;

		const maxEndCol = model ? model.getLineMaxColumn(endLineNumber) : Infinity;
		// End column should be at least startColumn + 1 to provide a visible squiggle
		const rawEndCol = Math.max(startColumn + 1, d.end_col || startColumn + 1);
		const endColumn = model ? Math.min(maxEndCol, Math.max(startColumn + 1, rawEndCol)) : rawEndCol;

		let severity: number;
		switch (d.severity) {
			case 'error':
				severity = MonacoMarkerSeverity.Error;
				break;
			case 'warning':
				severity = MonacoMarkerSeverity.Warning;
				break;
			case 'info':
			default:
				severity = MonacoMarkerSeverity.Info;
				break;
		}

		return {
			startLineNumber,
			startColumn,
			endLineNumber,
			endColumn,
			severity,
			message: d.message,
			source: 'Lean 4'
		};
	});
}
