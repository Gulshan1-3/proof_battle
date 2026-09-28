import type { editor, IPosition, IRange } from 'monaco-editor';

/**
 * Lean 4 Unicode input substitution map.
 * Triggered on backslash sequences upon pressing Space or Tab (identical to VS Code Lean 4 extension).
 */
export const LEAN_UNICODE_MAP: Record<string, string> = {
	forall: '∀',
	fa: '∀',
	exists: '∃',
	ex: '∃',
	to: '→',
	r: '→',
	from: '←',
	l: '←',
	iff: '↔',
	and: '∧',
	or: '∨',
	not: '¬',
	ne: '≠',
	le: '≤',
	ge: '≥',
	lt: '<',
	gt: '>',
	nat: 'ℕ',
	int: 'ℤ',
	real: 'ℝ',
	rat: 'ℚ',
	complex: 'ℂ',
	langle: '⟨',
	rangle: '⟩',
	vdash: '⊢',
	bot: '⊥',
	top: '⊤',
	alpha: 'α',
	beta: 'β',
	gamma: 'γ',
	lambda: 'λ',
	pi: 'π',
	sigma: 'σ',
	in: '∈',
	notin: '∉',
	sub: '⊆',
	sup: '⊇',
	cup: '∪',
	cap: '∩',
	mapsto: '↦',
	circ: '∘',
	comp: '∘',
	sum: '∑',
	prod: '∏',
	subset: '⊆',
	supset: '⊇',
	union: '∪',
	inter: '∩',
	empty: '∅'
};

export interface UnicodeExpansion {
	range: IRange;
	text: string;
}

/**
 * Pure function to inspect the text before `position` in `model` for a `\keyword` sequence.
 *
 * If a valid keyword is found, returns the range of `\keyword` to replace and the substitution character.
 * Range calculation:
 * - `startColumn = position.column - match[0].length` (covers the leading backslash)
 * - `endColumn = position.column`
 */
export function expandAt(
	model: { getLineContent(lineNumber: number): string },
	position: IPosition
): UnicodeExpansion | null {
	const lineContent = model.getLineContent(position.lineNumber);
	const textBefore = lineContent.substring(0, position.column - 1);

	const match = textBefore.match(/\\([a-zA-Z]+)$/);
	if (!match) return null;

	const keyword = match[1]!.toLowerCase();
	const replacement = LEAN_UNICODE_MAP[keyword];
	if (!replacement) return null;

	const startColumn = position.column - match[0].length;
	const endColumn = position.column;

	return {
		range: {
			startLineNumber: position.lineNumber,
			startColumn,
			endLineNumber: position.lineNumber,
			endColumn
		},
		text: replacement
	};
}

/**
 * Installs the backslash Unicode expansion listener on a Monaco editor instance.
 * Triggers on Space and Tab keydown events, replacing `\keyword` before standard editor insertion.
 */
export function installUnicodeMapper(codeEditor: editor.IStandaloneCodeEditor): {
	dispose: () => void;
} {
	const disposable = codeEditor.onKeyDown((e) => {
		if (e.code === 'Space' || e.code === 'Tab') {
			const model = codeEditor.getModel();
			const pos = codeEditor.getPosition();
			if (!model || !pos) return;

			const expansion = expandAt(model, pos);
			if (expansion) {
				e.preventDefault();
				e.stopPropagation();

				codeEditor.executeEdits('unicode-mapper', [
					{
						range: expansion.range,
						text: expansion.text,
						forceMoveMarkers: true
					}
				]);
			}
		}
	});

	return disposable;
}
