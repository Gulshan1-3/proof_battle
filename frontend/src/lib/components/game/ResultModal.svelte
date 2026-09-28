<script lang="ts">
	import Modal from '$lib/components/ui/Modal.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import type { MatchResult } from '$lib/stores/game.svelte';

	interface Props {
		open?: boolean;
		result: MatchResult;
		onrematch?: () => void;
		onnewmatch?: () => void;
		onclose?: () => void;
	}

	let { open = $bindable(false), result, onrematch, onnewmatch, onclose }: Props = $props();

	const isWin = $derived(result.outcome === 'Won' || result.outcome === 'ForfeitWin');
	const isLoss = $derived(result.outcome === 'Lost' || result.outcome === 'ForfeitLoss');

	const durationFormatted = $derived.by(() => {
		const totalSecs = Math.round(Number(result.durationMs) / 1000);
		const mins = Math.floor(totalSecs / 60);
		const secs = totalSecs % 60;
		return mins > 0 ? `${mins}m ${secs}s` : `${secs}s`;
	});

	$effect(() => {
		if (open && isWin) {
			const reducedMotion =
				typeof window !== 'undefined' &&
				window.matchMedia('(prefers-reduced-motion: reduce)').matches;

			if (!reducedMotion) {
				import('canvas-confetti').then(({ default: confetti }) => {
					confetti({
						particleCount: 80,
						spread: 70,
						origin: { y: 0.6 },
						colors: ['#7c6af7', '#34d399', '#9d8fff', '#ffffff']
					});
				});
			}
		}
	});
</script>

<Modal bind:open title="Match Concluded" {onclose}>
	<div class="flex flex-col items-center space-y-4 py-2 text-center">
		<!-- Trophy or outcome badge -->
		<div
			class="inline-flex h-16 w-16 items-center justify-center rounded-full {isWin
				? 'bg-[rgba(52,211,153,0.15)] text-[var(--success)]'
				: isLoss
					? 'bg-[rgba(248,113,113,0.15)] text-[var(--error)]'
					: 'bg-[var(--bg-elevated)] text-[var(--text-secondary)]'} shadow-inner"
		>
			{#if isWin}
				<svg
					width="32"
					height="32"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
				>
					<path d="M6 9H4.5a2.5 2.5 0 0 1 0-5H6"></path>
					<path d="M18 9h1.5a2.5 2.5 0 0 0 0-5H18"></path>
					<path d="M4 22h16"></path>
					<path d="M10 14.66V17c0 .55-.45 1-1 1H7v4h10v-4h-2c-.55 0-1-.45-1-1v-2.34"></path>
					<path d="M18 4H6v7a6 6 0 0 0 12 0V4z"></path>
				</svg>
			{:else}
				<svg
					width="32"
					height="32"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
				>
					<circle cx="12" cy="12" r="10"></circle>
					<line x1="15" y1="9" x2="9" y2="15"></line>
					<line x1="9" y1="9" x2="15" y2="15"></line>
				</svg>
			{/if}
		</div>

		<div>
			<h3 class="text-2xl font-black tracking-tight text-[var(--text-primary)]">
				{isWin ? 'VICTORY' : isLoss ? 'DEFEAT' : 'DRAW'}
			</h3>
			<p class="mt-0.5 text-xs tracking-widest text-[var(--text-secondary)] uppercase">
				Outcome: {result.outcome}
			</p>
		</div>

		<!-- Rating & Match Stats Grid -->
		<div class="grid w-full max-w-sm grid-cols-2 gap-3 pt-2">
			<div class="panel-elevated p-3 text-center">
				<span class="block text-[11px] tracking-wider text-[var(--text-secondary)] uppercase"
					>Rating Change</span
				>
				<span
					class="font-mono text-lg font-bold {result.eloDelta >= 0
						? 'text-[var(--success)]'
						: 'text-[var(--error)]'}"
				>
					{result.eloDelta >= 0 ? `+${result.eloDelta}` : result.eloDelta}
				</span>
			</div>
			<div class="panel-elevated p-3 text-center">
				<span class="block text-[11px] tracking-wider text-[var(--text-secondary)] uppercase"
					>Duration</span
				>
				<span class="font-mono text-lg font-bold text-[var(--text-primary)]">
					{durationFormatted}
				</span>
			</div>
		</div>

		<!-- Winning & Canonical Proofs -->
		{#if result.winningProof}
			<div class="w-full pt-2 text-left">
				<span
					class="mb-1 block text-xs font-semibold tracking-wider text-[var(--text-secondary)] uppercase"
				>
					Winning Proof
				</span>
				<pre
					class="panel-elevated max-h-32 overflow-x-auto p-3 font-mono text-xs text-[var(--success)]"><code
						>{result.winningProof}</code
					></pre>
			</div>
		{/if}

		{#if result.canonicalProof}
			<div class="w-full text-left">
				<span
					class="mb-1 block text-xs font-semibold tracking-wider text-[var(--text-secondary)] uppercase"
				>
					Canonical Solution
				</span>
				<pre
					class="panel-elevated max-h-32 overflow-x-auto p-3 font-mono text-xs text-[var(--text-primary)]"><code
						>{result.canonicalProof}</code
					></pre>
			</div>
		{/if}
	</div>

	{#snippet footer()}
		<Button variant="secondary" onclick={onrematch}>Rematch</Button>
		<Button variant="primary" onclick={onnewmatch}>New Opponent</Button>
	{/snippet}
</Modal>
