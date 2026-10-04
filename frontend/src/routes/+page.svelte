<script lang="ts">
	import { goto } from '$app/navigation';
	import Button from '$lib/components/ui/Button.svelte';
	import Badge from '$lib/components/ui/Badge.svelte';
	import { game } from '$lib/stores/game.svelte';
	import { wsClient } from '$lib/ws/client';
	import { MockWebSocket } from '$lib/mock/server';

	let username = $state('MathPro');

	$effect(() => {
		if (typeof window !== 'undefined') {
			const params = new URLSearchParams(window.location.search);
			if (params.has('mock') || sessionStorage.getItem('proofbattle.mock') === '1') {
				sessionStorage.setItem('proofbattle.mock', '1');
				MockWebSocket.install();
			}

			const saved = localStorage.getItem('proofbattle.username');
			if (saved) {
				username = saved;
				game.username = saved;
			}

			if (wsClient.state === 'disconnected') {
				wsClient.connect();
			}
		}
	});

	function handlePlayNow() {
		localStorage.setItem('proofbattle.username', username);
		game.username = username;
		const isMock =
			typeof sessionStorage !== 'undefined' && sessionStorage.getItem('proofbattle.mock') === '1';
		goto('/lobby' + (isMock ? '?mock=1' : ''));
	}

	function handlePractice() {
		localStorage.setItem('proofbattle.username', username);
		game.username = username;
		const isMock =
			typeof sessionStorage !== 'undefined' && sessionStorage.getItem('proofbattle.mock') === '1';
		goto('/lobby?practice=1' + (isMock ? '&mock=1' : ''));
	}

	function handlePrivateDuel() {
		localStorage.setItem('proofbattle.username', username);
		game.username = username;
		const isMock =
			typeof sessionStorage !== 'undefined' && sessionStorage.getItem('proofbattle.mock') === '1';
		goto('/lobby?private=1' + (isMock ? '&mock=1' : ''));
	}
</script>

<div class="mx-auto flex w-full max-w-5xl flex-1 flex-col items-center justify-between p-6">
	<!-- Top brand bar -->
	<header class="flex w-full items-center justify-between border-b border-[var(--bg-border)] py-4">
		<div class="flex items-center gap-3">
			<div
				class="flex h-9 w-9 items-center justify-center rounded-[var(--radius-md)] bg-[var(--accent)] font-mono text-lg font-bold text-white shadow-sm"
			>
				⊢
			</div>
			<div>
				<h1 class="text-lg font-black tracking-tight text-[var(--text-primary)]">ProofBattle</h1>
				<p class="text-[11px] font-semibold tracking-wider text-[var(--text-muted)] uppercase">
					Competitive Lean 4 Arena
				</p>
			</div>
		</div>

		<div class="flex items-center gap-3">
			<a
				href="/history"
				class="rounded-[var(--radius-md)] border border-[var(--bg-border)] px-3 py-1.5 text-xs font-semibold text-[var(--text-secondary)] transition-colors hover:border-[var(--accent)] hover:text-[var(--text-primary)]"
			>
				Match History
			</a>
			<Badge variant="accent">Season 1</Badge>
			<div class="font-mono text-xs text-[var(--text-secondary)]">
				Rating: <strong class="text-[var(--text-primary)]">{game.elo}</strong>
			</div>
		</div>
	</header>

	<!-- Hero & Match Queue Hub (Chess.com / Matics inspired) -->
	<section
		class="my-10 flex w-full max-w-xl flex-1 flex-col items-center justify-center text-center"
	>
		<div
			class="mb-6 inline-flex items-center gap-2 rounded-full border border-[var(--bg-border)] bg-[var(--bg-surface)] px-3 py-1 text-xs text-[var(--accent-bright)] shadow-sm"
		>
			<span class="h-2 w-2 animate-pulse rounded-full bg-[var(--success)]"></span>
			Real-Time Formal Verification Engine
		</div>

		<h2
			class="mb-4 text-4xl leading-tight font-black tracking-tight text-[var(--text-primary)] sm:text-5xl"
		>
			Prove Fast.<br />
			<span class="text-[var(--accent-bright)]">Outwit Opponents.</span>
		</h2>

		<p class="mb-8 max-w-md text-sm leading-relaxed text-[var(--text-secondary)]">
			Solve verified Lean 4 mathematics challenges in 1v1 synchronized tactical duels. Zero partial
			credit. Machine-verified soundness.
		</p>

		<!-- Player Setup Card -->
		<div class="panel mb-6 w-full space-y-5 p-6 text-left">
			<div>
				<label
					for="username-input"
					class="mb-1.5 block text-xs font-semibold tracking-wider text-[var(--text-secondary)] uppercase"
				>
					Enter Combatant Handle
				</label>
				<div class="relative">
					<input
						id="username-input"
						type="text"
						bind:value={username}
						placeholder="e.g. Hilbert_99"
						class="w-full rounded-[var(--radius-md)] border border-[var(--bg-border)] bg-[var(--bg-elevated)] px-4 py-2.5 font-mono text-sm text-[var(--text-primary)] placeholder-[var(--text-muted)] focus:border-[var(--accent)] focus:outline-none"
					/>
				</div>
			</div>

			<div class="grid grid-cols-1 gap-3 pt-2 sm:grid-cols-3">
				<Button variant="primary" size="lg" onclick={handlePlayNow} class="w-full">
					<svg
						width="18"
						height="18"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
					>
						<polygon points="5 3 19 12 5 21 5 3"></polygon>
					</svg>
					Play Ranked
				</Button>
				<Button variant="secondary" size="lg" onclick={handlePrivateDuel} class="w-full">
					Private Duel
				</Button>
				<Button variant="secondary" size="lg" onclick={handlePractice} class="w-full">
					Practice Solo
				</Button>
			</div>
		</div>
	</section>

	<!-- Live Stats Telemetry Strip -->
	<footer class="grid w-full grid-cols-3 gap-4 border-t border-[var(--bg-border)] py-4 text-center">
		<div class="panel-elevated px-2 py-3">
			<span class="block text-[11px] font-medium tracking-wider text-[var(--text-muted)] uppercase"
				>Live Battles</span
			>
			<span class="font-mono text-lg font-bold text-[var(--text-primary)]">14</span>
		</div>
		<div class="panel-elevated px-2 py-3">
			<span class="block text-[11px] font-medium tracking-wider text-[var(--text-muted)] uppercase"
				>Provers Online</span
			>
			<span class="font-mono text-lg font-bold text-[var(--success)]">68</span>
		</div>
		<div class="panel-elevated px-2 py-3">
			<span class="block text-[11px] font-medium tracking-wider text-[var(--text-muted)] uppercase"
				>Top Elo</span
			>
			<span class="font-mono text-lg font-bold text-[var(--accent-bright)]">2240</span>
		</div>
	</footer>
</div>
