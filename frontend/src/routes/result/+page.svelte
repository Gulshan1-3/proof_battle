<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import Button from '$lib/components/ui/Button.svelte';
	import { game, type MatchResult } from '$lib/stores/game.svelte';
	import { wsClient } from '$lib/ws/client';

	// Default fallback result for direct navigation or preview
	const result: MatchResult = $derived(
		game.result || {
			roomId: 'room-demo',
			outcome: 'Won',
			winnerId: 'you',
			winningProof: 'intro n\nsimp',
			canonicalProof: 'intro n\nsimp [Nat.add_zero]',
			eloDelta: 24,
			durationMs: 42000n,
			seq: 2n
		}
	);

	const isWin = $derived(result.outcome === 'Won' || result.outcome === 'ForfeitWin');
	const isLoss = $derived(result.outcome === 'Lost' || result.outcome === 'ForfeitLoss');

	const durationFormatted = $derived.by(() => {
		const totalSecs = Math.round(Number(result.durationMs) / 1000);
		const mins = Math.floor(totalSecs / 60);
		const secs = totalSecs % 60;
		return mins > 0 ? `${mins}m ${secs}s` : `${secs}s`;
	});

	onMount(() => {
		if (isWin) {
			const reducedMotion =
				typeof window !== 'undefined' &&
				window.matchMedia('(prefers-reduced-motion: reduce)').matches;

			if (!reducedMotion) {
				import('canvas-confetti').then(({ default: confetti }) => {
					confetti({
						particleCount: 100,
						spread: 80,
						origin: { y: 0.5 },
						colors: ['#7c6af7', '#34d399', '#9d8fff', '#ffffff']
					});
				});
			}
		}
	});

	function handleRematch() {
		wsClient.joinQueue();
		const isMock =
			typeof sessionStorage !== 'undefined' && sessionStorage.getItem('proofbattle.mock') === '1';
		goto('/lobby' + (isMock ? '?mock=1' : ''));
	}

	function handleNewOpponent() {
		wsClient.leaveQueue();
		const isMock =
			typeof sessionStorage !== 'undefined' && sessionStorage.getItem('proofbattle.mock') === '1';
		goto('/lobby' + (isMock ? '?mock=1' : ''));
	}

	function handlePractice() {
		const isMock =
			typeof sessionStorage !== 'undefined' && sessionStorage.getItem('proofbattle.mock') === '1';
		goto('/' + (isMock ? '?mock=1' : ''));
	}
</script>

<div class="mx-auto flex w-full max-w-2xl flex-1 flex-col items-center justify-center p-6">
	<div class="panel w-full space-y-6 p-8 text-center">
		<!-- Trophy Icon -->
		<div
			class="inline-flex h-20 w-20 items-center justify-center rounded-full {isWin
				? 'bg-[rgba(52,211,153,0.15)] text-[var(--success)]'
				: isLoss
					? 'bg-[rgba(248,113,113,0.15)] text-[var(--error)]'
					: 'bg-[var(--bg-elevated)] text-[var(--text-secondary)]'} mx-auto shadow-inner"
		>
			{#if isWin}
				<svg
					width="40"
					height="40"
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
					width="40"
					height="40"
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
			<h2 class="text-3xl font-black tracking-tight text-[var(--text-primary)]">
				{isWin ? 'MATCH VICTORY' : isLoss ? 'MATCH DEFEAT' : 'MATCH DRAW'}
			</h2>
			<p class="mt-1 text-xs tracking-widest text-[var(--text-secondary)] uppercase">
				Verdict: {result.outcome}
			</p>
		</div>

		<!-- Rating & Telemetry Grid -->
		<div class="mx-auto grid max-w-sm grid-cols-2 gap-4">
			<div class="panel-elevated p-4 text-center">
				<span class="mb-1 block text-xs tracking-wider text-[var(--text-secondary)] uppercase"
					>Rating Change</span
				>
				<span
					class="font-mono text-2xl font-bold {result.eloDelta >= 0
						? 'text-[var(--success)]'
						: 'text-[var(--error)]'}"
				>
					{result.eloDelta >= 0 ? `+${result.eloDelta}` : result.eloDelta}
				</span>
			</div>

			<div class="panel-elevated p-4 text-center">
				<span class="mb-1 block text-xs tracking-wider text-[var(--text-secondary)] uppercase"
					>Match Duration</span
				>
				<span class="font-mono text-2xl font-bold text-[var(--text-primary)]">
					{durationFormatted}
				</span>
			</div>
		</div>

		<!-- Proof Solutions Cards -->
		<div class="space-y-4 pt-2 text-left">
			{#if result.winningProof}
				<div>
					<span
						class="mb-1 block text-xs font-semibold tracking-wider text-[var(--text-secondary)] uppercase"
					>
						Winning Proof
					</span>
					<pre
						class="panel-elevated overflow-x-auto p-4 font-mono text-xs text-[var(--success)]"><code
							>{result.winningProof}</code
						></pre>
				</div>
			{/if}

			{#if result.canonicalProof}
				<div>
					<span
						class="mb-1 block text-xs font-semibold tracking-wider text-[var(--text-secondary)] uppercase"
					>
						Canonical Solution
					</span>
					<pre
						class="panel-elevated overflow-x-auto p-4 font-mono text-xs text-[var(--text-primary)]"><code
							>{result.canonicalProof}</code
						></pre>
				</div>
			{/if}
		</div>

		<!-- Navigation / Rematch CTAs -->
		<div class="flex items-center justify-center gap-3 border-t border-[var(--bg-border)] pt-4">
			<Button variant="secondary" onclick={handlePractice}>Home Arena</Button>
			<Button variant="secondary" onclick={handleRematch}>Rematch</Button>
			<Button variant="primary" onclick={handleNewOpponent}>New Opponent</Button>
		</div>
	</div>
</div>
