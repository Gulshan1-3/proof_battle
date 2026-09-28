<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import type { Diagnostic } from '$lib/ws/generated';
	import type { editor as MonacoEditorType } from 'monaco-editor';
	import { toMonacoMarkers } from '$lib/utils/diagnostics';
	import { installUnicodeMapper } from './unicode-mapper';

	interface Props {
		value?: string;
		readonly?: boolean;
		diagnostics?: Diagnostic[];
		onchange?: (value: string) => void;
	}

	let { value = $bindable(''), readonly = false, diagnostics = [], onchange }: Props = $props();

	let container = $state<HTMLDivElement | null>(null);
	let editorInstance: MonacoEditorType.IStandaloneCodeEditor | null = null;
	let monacoInstance: typeof import('monaco-editor') | null = null;
	let unicodeDisposable: { dispose: () => void } | null = null;

	onMount(async () => {
		if (!container) return;

		// Dynamically import Monaco on the client only (ADR-011)
		const { loadMonaco } = await import('./monaco');
		const monaco = await loadMonaco();
		monacoInstance = monaco;

		editorInstance = monaco.editor.create(container, {
			value,
			language: 'lean4',
			theme: 'proofbattle-dark',
			fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace",
			fontSize: 14,
			lineHeight: 22,
			minimap: { enabled: false },
			scrollBeyondLastLine: false,
			wordWrap: 'on',
			lineNumbers: 'on',
			renderLineHighlight: 'all',
			automaticLayout: true,
			readOnly: readonly,
			overviewRulerLanes: 0,
			guides: {
				indentation: true
			},
			cursorBlinking: 'smooth',
			cursorSmoothCaretAnimation: 'on',
			tabSize: 2,
			insertSpaces: true,
			quickSuggestions: true,
			fixedOverflowWidgets: true,
			stickyScroll: { enabled: false },
			unicodeHighlight: {
				ambiguousCharacters: false
			}
		});

		// Listen to content changes and invoke onchange
		editorInstance.onDidChangeModelContent(() => {
			if (!editorInstance) return;
			const currentVal = editorInstance.getValue();
			if (currentVal !== value) {
				value = currentVal;
				onchange?.(currentVal);
			}
		});

		// Install backslash-to-Unicode keyboard substitution
		unicodeDisposable = installUnicodeMapper(editorInstance);

		// Apply initial markers
		applyMarkers(diagnostics);
	});

	function applyMarkers(diags: Diagnostic[]) {
		if (!editorInstance || !monacoInstance) return;
		const model = editorInstance.getModel();
		if (!model) return;

		// Convert and clamp markers to avoid throwing on out-of-range lines
		const markers = toMonacoMarkers(diags, model);
		monacoInstance.editor.setModelMarkers(model, 'lean4', markers);
	}

	// Reactive effect for diagnostics changes: updates markers, or clears when empty
	$effect(() => {
		applyMarkers(diagnostics);
	});

	// Reactive effect for external value prop changes
	$effect(() => {
		if (editorInstance && value !== editorInstance.getValue()) {
			editorInstance.setValue(value);
		}
	});

	// Reactive effect for readonly property
	$effect(() => {
		if (editorInstance) {
			editorInstance.updateOptions({ readOnly: readonly });
		}
	});

	onDestroy(() => {
		unicodeDisposable?.dispose();
		if (editorInstance) {
			editorInstance.dispose();
			editorInstance = null;
		}
	});

	// Expose imperative API for parent components
	export function getValue(): string {
		return editorInstance ? editorInstance.getValue() : value;
	}

	export function focus(): void {
		editorInstance?.focus();
	}

	export function insertAtCursor(text: string): void {
		if (!editorInstance) {
			value += text;
			return;
		}
		const selection = editorInstance.getSelection();
		if (selection) {
			editorInstance.executeEdits('insert-symbol', [
				{
					range: selection,
					text,
					forceMoveMarkers: true
				}
			]);
		} else {
			editorInstance.trigger('keyboard', 'type', { text });
		}
		editorInstance.focus();
	}

	export function setPosition(lineNumber: number, column: number = 1): void {
		if (!editorInstance) return;
		editorInstance.setPosition({ lineNumber, column });
		editorInstance.revealLineInCenter(lineNumber);
		editorInstance.focus();
	}
</script>

<div
	bind:this={container}
	class="relative h-full w-full overflow-hidden bg-[var(--editor-bg)]"
	data-testid="monaco-editor-container"
></div>
