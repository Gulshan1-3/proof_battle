import type { editor } from 'monaco-editor';

/**
 * ProofBattle Dark Editor Theme
 * Calibrated against --editor-bg: #111113.
 *
 * Contrast Audit against #111113:
 * - keyword.declaration (#c792ea): 7.84:1 (PASS >= 4.5:1)
 * - keyword.control     (#7c6af7): 4.73:1 (PASS >= 4.5:1)
 * - keyword.operator    (#89ddff): 12.44:1 (PASS >= 4.5:1)
 * - keyword             (#c792ea): 7.84:1 (PASS >= 4.5:1)
 * - type.identifier     (#ffcb6b): 12.58:1 (PASS >= 4.5:1)
 * - comment             (#8b8b9e): 5.64:1 (PASS >= 4.5:1; design doc's default #4a4a5a was 2.17:1 FAIL)
 * - number              (#f78c6c): 8.01:1 (PASS >= 4.5:1)
 * - string              (#c3e88d): 13.69:1 (PASS >= 4.5:1)
 * - identifier          (#f0f0f5): 16.60:1 (PASS >= 4.5:1)
 */
export function getEditorTheme(): editor.IStandaloneThemeData {
	return {
		base: 'vs-dark',
		inherit: true,
		rules: [
			{ token: 'keyword.declaration', foreground: 'c792ea', fontStyle: 'bold' },
			{ token: 'keyword.control', foreground: '7c6af7', fontStyle: 'bold' },
			{ token: 'keyword.operator', foreground: '89ddff' },
			{ token: 'keyword.special', foreground: 'f07178', fontStyle: 'italic' },
			{ token: 'keyword', foreground: 'c792ea' },
			{ token: 'type.identifier', foreground: 'ffcb6b' },
			{ token: 'comment', foreground: '8b8b9e', fontStyle: 'italic' },
			{ token: 'number', foreground: 'f78c6c' },
			{ token: 'string', foreground: 'c3e88d' },
			{ token: 'string.escape', foreground: 'eeffff' },
			{ token: 'identifier', foreground: 'f0f0f5' }
		],
		colors: {
			'editor.background': '#111113',
			'editor.foreground': '#f0f0f5',
			'editor.lineHighlightBackground': '#1e1e26',
			'editor.selectionBackground': '#7c6af740',
			'editorCursor.foreground': '#7c6af7',
			'editorLineNumber.foreground': '#4a4a5a',
			'editorLineNumber.activeForeground': '#8b8b9e',
			'editorGutter.background': '#1a1a1f',
			'editorWidget.background': '#1c1c21',
			'editorSuggestWidget.background': '#1c1c21',
			'editorSuggestWidget.border': '#2a2a31'
		}
	};
}
