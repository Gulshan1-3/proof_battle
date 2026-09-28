<script lang="ts">
	interface Props {
		oninsert: (symbol: string) => void;
	}

	let { oninsert }: Props = $props();

	export const PALETTE_SYMBOLS = [
		'∀',
		'∃',
		'→',
		'↔',
		'∧',
		'∨',
		'¬',
		'≠',
		'≤',
		'≥',
		'ℕ',
		'ℤ',
		'ℚ',
		'ℝ',
		'ℂ',
		'⟨',
		'⟩',
		'⊢',
		'⊥',
		'⊤',
		'↦',
		'∘',
		'∈',
		'∉'
	];
</script>

<div
	class="flex flex-wrap items-center gap-1 border-b border-[var(--bg-border)] bg-[var(--bg-surface)] px-3 py-1.5"
	role="toolbar"
	aria-label="Lean 4 Unicode Symbol Palette"
>
	<span
		class="mr-2 font-mono text-[10px] font-bold tracking-wider text-[var(--text-muted)] uppercase select-none"
	>
		Symbols:
	</span>
	<div class="flex flex-wrap items-center gap-1">
		{#each PALETTE_SYMBOLS as symbol (symbol)}
			<button
				type="button"
				class="flex h-6 min-w-6 cursor-pointer items-center justify-center rounded px-1 font-mono text-xs text-[var(--text-primary)] transition-colors hover:bg-[var(--accent)] hover:text-white active:scale-95"
				title="Insert {symbol}"
				onmousedown={(e) => e.preventDefault()}
				onclick={() => oninsert(symbol)}
			>
				{symbol}
			</button>
		{/each}
	</div>
</div>
