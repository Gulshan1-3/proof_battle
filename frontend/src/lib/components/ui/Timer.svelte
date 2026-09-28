<script lang="ts">
	import { onMount } from 'svelte';

	interface Props {
		endsAtMs: number;
		serverTimeOffsetMs?: number;
		active?: boolean;
		onexpire?: () => void;
		class?: string;
	}

	let {
		endsAtMs,
		serverTimeOffsetMs = 0,
		active = true,
		onexpire,
		class: className = ''
	}: Props = $props();

	let now = $state(Date.now());
	let expiredTriggered = false;

	const remainingMs = $derived.by(() => {
		if (!active || endsAtMs <= 0) return 0;
		const currentServerTime = now + serverTimeOffsetMs;
		return Math.max(0, endsAtMs - currentServerTime);
	});

	const remainingSecs = $derived(Math.ceil(remainingMs / 1000));
	const isUrgent = $derived(remainingSecs <= 60 && remainingSecs > 10);
	const isCritical = $derived(remainingSecs <= 10 && remainingSecs > 0);

	const formattedTime = $derived.by(() => {
		const mins = Math.floor(remainingSecs / 60);
		const secs = remainingSecs % 60;
		const pad = (n: number) => n.toString().padStart(2, '0');
		return `${pad(mins)}:${pad(secs)}`;
	});

	onMount(() => {
		const interval = setInterval(() => {
			now = Date.now();
			if (active && endsAtMs > 0 && remainingMs <= 0 && !expiredTriggered) {
				expiredTriggered = true;
				onexpire?.();
			}
		}, 250);

		return () => clearInterval(interval);
	});
</script>

<div
	class="inline-flex items-center gap-2 rounded-[var(--radius-md)] border px-3 py-1.5 font-mono text-base font-bold tracking-wider transition-colors duration-200 {className}"
	class:bg-[var(--bg-elevated)]={!isUrgent && !isCritical}
	class:border-[var(--bg-border)]={!isUrgent && !isCritical}
	class:text-[var(--text-primary)]={!isUrgent && !isCritical}
	class:bg-[rgba(251,191,36,0.1)]={isUrgent}
	class:border-[var(--warning)]={isUrgent}
	class:text-[var(--warning)]={isUrgent}
	class:bg-[rgba(248,113,113,0.15)]={isCritical}
	class:border-[var(--error)]={isCritical}
	class:text-[var(--error)]={isCritical}
	class:timer-critical={isCritical}
	role="timer"
	aria-label="Time remaining: {formattedTime}"
>
	<svg
		width="16"
		height="16"
		viewBox="0 0 24 24"
		fill="none"
		stroke="currentColor"
		stroke-width="2"
		stroke-linecap="round"
		stroke-linejoin="round"
	>
		<circle cx="12" cy="12" r="10"></circle>
		<polyline points="12 6 12 12 16 14"></polyline>
	</svg>
	<span>{formattedTime}</span>
</div>
