<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		variant?: 'default' | 'success' | 'error' | 'warning' | 'info' | 'accent';
		size?: 'sm' | 'md';
		children?: Snippet;
		class?: string;
	}

	let { variant = 'default', size = 'md', children, class: className = '' }: Props = $props();

	const variantStyles = {
		default:
			'bg-[var(--bg-elevated)] text-[var(--text-secondary)] border border-[var(--bg-border)]',
		success:
			'bg-[rgba(52,211,153,0.12)] text-[var(--success)] border border-[rgba(52,211,153,0.3)]',
		error: 'bg-[rgba(248,113,113,0.12)] text-[var(--error)] border border-[rgba(248,113,113,0.3)]',
		warning:
			'bg-[rgba(251,191,36,0.12)] text-[var(--warning)] border border-[rgba(251,191,36,0.3)]',
		info: 'bg-[rgba(96,165,250,0.12)] text-[var(--info)] border border-[rgba(96,165,250,0.3)]',
		accent:
			'bg-[var(--accent-dim)] text-[var(--accent-bright)] border border-[rgba(124,106,247,0.3)]'
	};

	const sizeStyles = {
		sm: 'text-[11px] px-1.5 py-0.5 rounded-[var(--radius-sm)]',
		md: 'text-xs px-2.5 py-1 rounded-[var(--radius-md)]'
	};
</script>

<span
	class="inline-flex items-center font-[var(--font-ui)] font-medium tracking-wide uppercase {sizeStyles[
		size
	]} {variantStyles[variant]} {className}"
>
	{@render children?.()}
</span>
