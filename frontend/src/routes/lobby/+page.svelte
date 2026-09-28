<script lang="ts">
	import { goto } from '$app/navigation';
	import Button from '$lib/components/ui/Button.svelte';
	import Badge from '$lib/components/ui/Badge.svelte';
	import { game } from '$lib/stores/game.svelte';
	import { wsClient } from '$lib/ws/client';
	import { MockWebSocket } from '$lib/mock/server';

	let elapsedSecs = $state(0);
	const isPractice = $derived(
		typeof window !== 'undefined' &&
		new URLSearchParams(window.location.search).get('practice') === '1'
	);

	$effect(() => {
		// If mock param present or in sessionStorage, install mock WebSocket
		const params = new URLSearchParams(window.location.search);
		const isMock =
			params.has('mock') ||
			(typeof sessionStorage !== 'undefined' && sessionStorage.getItem('proofbattle.mock') === '1');
		if (isMock) {
			if (typeof sessionStorage !== 'undefined') sessionStorage.setItem('proofbattle.mock', '1');
			MockWebSocket.install();
		}

		if (wsClient.state === 'disconnected') {
			wsClient.connect();
		}

		function triggerJoin() {
			if (isPractice) {
				wsClient.practiceJoin();
			} else {
				wsClient.joinQueue();
			}
		}

		if (wsClient.state === 'connected_idle' || wsClient.state === 'game_ended') {
			triggerJoin();
		}

		// Join queue once connected or immediately
		const checkInterval = setInterval(() => {
			if (wsClient.state === 'connected_idle' || wsClient.state === 'game_ended') {
				triggerJoin();
				clearInterval(checkInterval);
			}
		}, 100);

		const timer = setInterval(() => {
			elapsedSecs += 1;
		}, 1000);

		return () => {
			clearInterval(checkInterval);
			clearInterval(timer);
		};
	});

	// Reactive redirect when matched
	$effect(() => {
		if (game.roomId && (wsClient.state === 'game_starting' || wsClient.state === 'in_game')) {
			const isMock =
				typeof sessionStorage !== 'undefined' && sessionStorage.getItem('proofbattle.mock') === '1';
			const params = new URLSearchParams();
			if (isMock) params.set('mock', '1');
			if (isPractice) params.set('practice', '1');
			const query = params.toString() ? '?' + params.toString() : '';
			goto('/game' + query);
		}
	});

	function handleCancel() {
		wsClient.leaveQueue();
		const isMock =
			typeof sessionStorage !== 'undefined' && sessionStorage.getItem('proofbattle.mock') === '1';
		goto('/' + (isMock ? '?mock=1' : ''));
	}
</script>

<div
	class="mx-auto flex w-full max-w-lg flex-1 flex-col items-center justify-center p-6 text-center"
>
	<!-- Radar pulse visualizer -->
	<div class="relative mb-8 flex h-36 w-36 items-center justify-center">
		<div class="pulse absolute inset-0 rounded-full border border-[var(--accent)] opacity-20"></div>
		<div
			class="absolute inset-4 animate-ping rounded-full border border-[var(--accent)] opacity-40"
		></div>
		<div
			class="relative flex h-20 w-20 items-center justify-center rounded-full border-2 border-[var(--accent)] bg-[var(--bg-surface)] shadow-lg"
		>
			<svg
				width="32"
				height="32"
				viewBox="0 0 24 24"
				fill="none"
				stroke="var(--accent-bright)"
				stroke-width="2"
			>
				<circle cx="12" cy="12" r="10"></circle>
				<line x1="12" y1="2" x2="12" y2="12"></line>
			</svg>
		</div>
	</div>

	<Badge variant="accent" class="mb-3">
		{isPractice ? 'Solo Practice Arena' : 'Matchmaking Arena'}
	</Badge>
	<h2 class="mb-2 text-2xl font-black tracking-tight text-[var(--text-primary)]">
		{isPractice ? 'Preparing Your Challenge...' : 'Finding Your Opponent...'}
	</h2>

	<p class="mb-6 font-mono text-sm text-[var(--text-secondary)]">
		{isPractice ? 'Setup Time' : 'Queue Time'}: {Math.floor(elapsedSecs / 60)}:{(elapsedSecs % 60).toString().padStart(2, '0')}
	</p>

	<!-- Telemetry Card -->
	<div class="panel mb-6 w-full space-y-3 p-5 text-left">
		{#if isPractice}
			<div class="flex items-center justify-between text-xs">
				<span class="font-medium text-[var(--text-secondary)]">Mode:</span>
				<span class="font-mono font-bold text-[var(--text-primary)]">Solo Sandbox</span>
			</div>
			<div class="flex items-center justify-between text-xs">
				<span class="font-medium text-[var(--text-secondary)]">Sparring Bot:</span>
				<span class="font-mono text-[var(--accent-bright)]">Lean Practice Bot (1200)</span>
			</div>
			<div class="flex items-center justify-between text-xs">
				<span class="font-medium text-[var(--text-secondary)]">Verification:</span>
				<span class="font-mono text-[var(--text-primary)]">Live Lean 4 Judge</span>
			</div>
			<div
				class="flex items-center gap-2 border-t border-[var(--bg-border)] pt-2 text-xs font-medium text-[var(--success)]"
			>
				<span class="h-2 w-2 animate-pulse rounded-full bg-[var(--success)]"></span>
				Spawning Sandbox Environment...
			</div>
		{:else}
			<div class="flex items-center justify-between text-xs">
				<span class="font-medium text-[var(--text-secondary)]">Your Rating:</span>
				<span class="font-mono font-bold text-[var(--text-primary)]">{game.elo} ELO</span>
			</div>

			<div class="flex items-center justify-between text-xs">
				<span class="font-medium text-[var(--text-secondary)]">Match Search Range:</span>
				<span class="font-mono text-[var(--accent-bright)]">
					{game.searchRange
						? `±${(game.searchRange.max_elo - game.searchRange.min_elo) / 2}`
						: '±150'}
				</span>
			</div>

			<div class="flex items-center justify-between text-xs">
				<span class="font-medium text-[var(--text-secondary)]">Players in Queue:</span>
				<span class="font-mono text-[var(--text-primary)]">{Math.max(1, game.queueSize)}</span>
			</div>

			<div
				class="flex items-center gap-2 border-t border-[var(--bg-border)] pt-2 text-xs font-medium text-[var(--success)]"
			>
				<span class="h-2 w-2 animate-pulse rounded-full bg-[var(--success)]"></span>
				Connected to Judge Cluster — Searching
			</div>
		{/if}
	</div>

	<Button variant="secondary" size="md" onclick={handleCancel} class="w-full">
		{isPractice ? 'Cancel Practice' : 'Cancel Matchmaking'}
	</Button>
</div>
