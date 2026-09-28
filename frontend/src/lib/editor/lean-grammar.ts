import type { languages } from 'monaco-editor';

export function getLean4Grammar(): languages.IMonarchLanguage {
	return {
		defaultToken: '',
		tokenPostfix: '.lean',

		keywords: [
			'theorem',
			'lemma',
			'def',
			'by',
			'intro',
			'apply',
			'exact',
			'simp',
			'ring',
			'omega',
			'linarith',
			'nlinarith',
			'norm_num',
			'have',
			'obtain',
			'cases',
			'induction',
			'constructor',
			'use',
			'rfl',
			'rw',
			'calc',
			'show',
			'suffices',
			'contradiction',
			'assumption',
			'trivial',
			'decide',
			'native_decide',
			'import',
			'open',
			'namespace',
			'end',
			'section',
			'variable',
			'fun',
			'match',
			'if',
			'then',
			'else',
			'let',
			'in',
			'do',
			'return',
			'pure',
			'Prop',
			'Type',
			'Sort',
			'where',
			'with',
			'extends',
			'structure',
			'class',
			'instance',
			'axiom',
			'opaque',
			'partial',
			'mutual'
		],

		typeKeywords: ['ℕ', 'ℤ', 'ℚ', 'ℝ', 'ℂ', 'Bool', 'Fin', 'List', 'Set', 'Option', 'Nat', 'Int'],

		operators: [
			':=',
			':',
			'::',
			'|',
			'⟨',
			'⟩',
			'←',
			'→',
			'↔',
			'∀',
			'∃',
			'∧',
			'∨',
			'¬',
			'⊢',
			'⊥',
			'⊤',
			'≤',
			'≥',
			'≠',
			'≡',
			'⟹',
			'↦',
			'∘',
			'∩',
			'∪',
			'⊆',
			'∈',
			'∉',
			'+',
			'-',
			'*',
			'/',
			'=',
			'<',
			'>'
		],

		tokenizer: {
			root: [
				// Single-line comments take highest precedence (e.g. `-- #eval` is a comment, not special)
				[/--.*$/, 'comment'],

				// Nested block comments
				[/\/-/, 'comment', '@blockComment'],

				// Commands and special macros (#check, #eval, etc.)
				[/#\w+/, 'keyword.special'],

				// Declaration keywords
				[/\b(theorem|lemma|def|axiom|structure|class|instance)\b/, 'keyword.declaration'],

				// Control keywords
				[/\b(by|do|if|then|else|match|with|fun)\b/, 'keyword.control'],

				// Multi-character and symbol operators
				[/:=|::|->|<-|==|!=|<=|>=|=>/, 'keyword.operator'],
				[/[∀∃∧∨¬→↔⊢⊥⊤≤≥≠≡⟹←↦∘∩∪⊆∈∉]/, 'keyword.operator'],
				[/[+\-*/=<>|:]/, 'keyword.operator'],

				// Math / Builtin Type identifiers
				[/[ℕℤℚℝℂ]/, 'type.identifier'],
				[/\b[A-Z][a-zA-Z0-9_']*\b/, 'type.identifier'],

				// Numbers
				[/\b\d+\b/, 'number'],

				// String literals with escape support
				[/"/, 'string', '@string'],
				[/`[^`]*`/, 'string.backtick'],

				// Identifiers and remaining keywords
				[
					/\b[a-z_][a-zA-Z0-9_']*\b/,
					{
						cases: {
							'@keywords': 'keyword',
							'@default': 'identifier'
						}
					}
				],

				// Delimiters
				[/[()[\]{}]/, '@brackets']
			],

			blockComment: [
				[/\/-/, 'comment', '@push'],
				[/-\//, 'comment', '@pop'],
				[/[\s\S]/, 'comment']
			],

			string: [
				[/[^\\"]+/, 'string'],
				[/\\(?:[nrt\\"]|x[0-9a-fA-F]{2}|u[0-9a-fA-F]{4})/, 'string.escape'],
				[/\\./, 'string.invalid'],
				[/"/, 'string', '@pop']
			]
		}
	};
}
