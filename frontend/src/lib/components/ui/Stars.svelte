<script lang="ts">
	interface Props {
		difficulty?: number; // 1 to 5
		max?: number;
		size?: number;
		class?: string;
	}

	let { difficulty = 1, max = 5, size = 14, class: className = '' }: Props = $props();

	const stars = $derived(Array.from({ length: max }, (_, i) => i < difficulty));
</script>

<div
	class="inline-flex items-center gap-1 {className}"
	aria-label="Difficulty: {difficulty} of {max}"
>
	{#each stars as filled, i (i)}
		<svg
			width={size}
			height={size}
			viewBox="0 0 24 24"
			fill={filled ? 'var(--warning)' : 'none'}
			stroke={filled ? 'var(--warning)' : 'var(--text-muted)'}
			stroke-width="2"
			stroke-linecap="round"
			stroke-linejoin="round"
			class="transition-colors duration-150"
		>
			<polygon
				points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"
			></polygon>
		</svg>
	{/each}
</div>
