<script lang="ts">
	import type { Snippet } from 'svelte';
	import Spinner from './Spinner.svelte';

	interface Props {
		variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
		size?: 'sm' | 'md' | 'lg';
		type?: 'button' | 'submit' | 'reset';
		disabled?: boolean;
		loading?: boolean;
		onclick?: (e: MouseEvent) => void;
		children?: Snippet;
		class?: string;
	}

	let {
		variant = 'primary',
		size = 'md',
		type = 'button',
		disabled = false,
		loading = false,
		onclick,
		children,
		class: className = ''
	}: Props = $props();

	const baseStyles =
		'inline-flex items-center justify-center font-semibold transition-all duration-150 cursor-pointer select-none focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--accent)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--bg-base)] disabled:opacity-50 disabled:cursor-not-allowed';

	const sizeStyles = {
		sm: 'text-xs px-2.5 py-1.5 rounded-[var(--radius-sm)] gap-1.5',
		md: 'text-sm px-4 py-2 rounded-[var(--radius-md)] gap-2',
		lg: 'text-base px-6 py-3 rounded-[var(--radius-lg)] gap-2.5'
	};

	const variantStyles = {
		primary:
			'bg-[var(--accent)] hover:bg-[var(--accent-bright)] text-white border border-[rgba(255,255,255,0.1)] active:scale-[0.98] shadow-sm',
		secondary:
			'bg-[var(--bg-surface)] hover:bg-[var(--bg-elevated)] text-[var(--text-primary)] border border-[var(--bg-border)] active:scale-[0.98]',
		ghost:
			'bg-transparent hover:bg-[rgba(255,255,255,0.06)] text-[var(--text-secondary)] hover:text-[var(--text-primary)]',
		danger:
			'bg-[var(--error)] hover:opacity-90 text-white border border-[rgba(0,0,0,0.2)] active:scale-[0.98]'
	};
</script>

<button
	{type}
	disabled={disabled || loading}
	onclick={(e) => {
		if (disabled || loading) {
			e.preventDefault();
			return;
		}
		onclick?.(e);
	}}
	class="{baseStyles} {sizeStyles[size]} {variantStyles[variant]} {className}"
>
	{#if loading}
		<Spinner size={size === 'sm' ? 14 : 16} />
	{/if}
	{@render children?.()}
</button>
