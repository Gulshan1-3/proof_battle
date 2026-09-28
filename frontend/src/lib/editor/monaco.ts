import type * as MonacoType from 'monaco-editor';
import { getLean4Grammar } from './lean-grammar';
import { getEditorTheme } from './editor-theme';

let monacoPromise: Promise<typeof MonacoType> | null = null;
let isLeanRegistered = false;

export const LEAN_TACTIC_COMPLETIONS = [
	{ label: 'intro', detail: 'tactic', doc: 'Introduce hypotheses or variables into the context' },
	{ label: 'exact', detail: 'tactic', doc: 'Close the goal by providing an exact proof term' },
	{ label: 'apply', detail: 'tactic', doc: 'Apply a theorem whose conclusion matches the goal' },
	{ label: 'simp', detail: 'tactic', doc: 'Simplify goal using simp lemmas' },
	{ label: 'ring', detail: 'tactic', doc: 'Solve ring equality goals over ℕ, ℤ, ℝ, etc.' },
	{ label: 'omega', detail: 'tactic', doc: 'Decision procedure for linear integer arithmetic' },
	{ label: 'linarith', detail: 'tactic', doc: 'Linear arithmetic solver over ordered fields' },
	{ label: 'nlinarith', detail: 'tactic', doc: 'Nonlinear arithmetic heuristic solver' },
	{ label: 'norm_num', detail: 'tactic', doc: 'Normalize numerical expressions and calculations' },
	{ label: 'rfl', detail: 'tactic', doc: 'Close reflexive equality goals (x = x)' },
	{ label: 'rw', detail: 'tactic', doc: 'Rewrite using an equality hypothesis or lemma' },
	{ label: 'have', detail: 'tactic', doc: 'Introduce an intermediate lemma or hypothesis' },
	{ label: 'obtain', detail: 'tactic', doc: 'Destructure an existential or conjunction' },
	{ label: 'cases', detail: 'tactic', doc: 'Perform case analysis on an inductive type' },
	{ label: 'induction', detail: 'tactic', doc: 'Proof by mathematical induction on a variable' },
	{ label: 'constructor', detail: 'tactic', doc: 'Apply constructor for conjunction or structure' },
	{ label: 'use', detail: 'tactic', doc: 'Instantiate an existential quantifier' },
	{ label: 'calc', detail: 'tactic', doc: 'Chain calculation equalities or inequalities' },
	{ label: 'show', detail: 'tactic', doc: 'Restate the current goal or target type' },
	{ label: 'suffices', detail: 'tactic', doc: 'Reduce current goal to a sufficient condition' },
	{ label: 'trivial', detail: 'tactic', doc: 'Close trivial goals with True.intro' },
	{ label: 'contradiction', detail: 'tactic', doc: 'Derive contradiction from context hypotheses' },
	{ label: 'decide', detail: 'tactic', doc: 'Decide decidable propositions' },
	{ label: 'assumption', detail: 'tactic', doc: 'Find a hypothesis matching the goal' },
	{ label: 'ext', detail: 'tactic', doc: 'Apply extensionality rules for functions or sets' },
	{ label: 'left', detail: 'tactic', doc: 'Select left disjunct of an or-goal' },
	{ label: 'right', detail: 'tactic', doc: 'Select right disjunct of an or-goal' },
	{ label: 'split', detail: 'tactic', doc: 'Split if-then-else or match expressions' },
	{ label: 'congr', detail: 'tactic', doc: 'Apply congruence rules to equality goals' },
	{ label: 'revert', detail: 'tactic', doc: 'Move a hypothesis back into the goal' }
];

function registerLean(monaco: typeof MonacoType) {
	if (isLeanRegistered) return;
	isLeanRegistered = true;

	monaco.languages.register({ id: 'lean4' });
	monaco.languages.setMonarchTokensProvider('lean4', getLean4Grammar());

	// Register static Lean 4 tactics completions (no LSP, static list per S8-P2)
	monaco.languages.registerCompletionItemProvider('lean4', {
		provideCompletionItems: (model, position) => {
			const word = model.getWordUntilPosition(position);
			const range = {
				startLineNumber: position.lineNumber,
				endLineNumber: position.lineNumber,
				startColumn: word.startColumn,
				endColumn: word.endColumn
			};

			const suggestions = LEAN_TACTIC_COMPLETIONS.map((item) => ({
				label: item.label,
				kind: monaco.languages.CompletionItemKind.Keyword,
				insertText: item.label,
				detail: item.detail,
				documentation: item.doc,
				range
			}));

			return { suggestions };
		}
	});
}

/**
 * Lazily loads Monaco Editor entirely client-side using a dynamic import.
 * Workers are instantiated via standard ES module Worker URLs.
 *
 * ADR-011: Zero CDN references. All assets are self-hosted.
 */
export async function loadMonaco(): Promise<typeof MonacoType> {
	if (!monacoPromise) {
		monacoPromise = (async () => {
			if (typeof window !== 'undefined') {
				(self as unknown as Record<string, unknown>).MonacoEnvironment = {
					getWorker: function () {
						return new Worker(
							URL.createObjectURL(
								new Blob(['self.onmessage = function () {};'], { type: 'application/javascript' })
							)
						);
					}
				};
			}

			const monaco = (await import('monaco-core')) as typeof MonacoType;

			registerLean(monaco);
			monaco.editor.defineTheme('proofbattle-dark', getEditorTheme());
			return monaco;
		})();
	}
	return monacoPromise;
}
