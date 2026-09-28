<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		content: string;
		position?: 'top' | 'bottom' | 'left' | 'right';
		children?: Snippet;
		class?: string;
	}

	let { content, position = 'top', children, class: className = '' }: Props = $props();

	let visible = $state(false);

	const positionStyles = {
		top: 'bottom-full left-1/2 -translate-x-1/2 mb-2',
		bottom: 'top-full left-1/2 -translate-x-1/2 mt-2',
		left: 'right-full top-1/2 -translate-y-1/2 mr-2',
		right: 'left-full top-1/2 -translate-y-1/2 ml-2'
	};
</script>

<div
	class="relative inline-block {className}"
	role="region"
	aria-label="Tooltip container"
	onmouseenter={() => (visible = true)}
	onmouseleave={() => (visible = false)}
	onfocusin={() => (visible = true)}
	onfocusout={() => (visible = false)}
>
	{@render children?.()}

	{#if visible}
		<div
			role="tooltip"
			class="pointer-events-none absolute z-50 rounded-[var(--radius-sm)] border border-[var(--bg-border)] bg-[var(--bg-overlay)] px-2.5 py-1.5 text-xs whitespace-nowrap text-[var(--text-primary)] shadow-lg {positionStyles[
				position
			]}"
		>
			{content}
		</div>
	{/if}
</div>
