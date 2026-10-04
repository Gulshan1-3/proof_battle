<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import Button from '$lib/components/ui/Button.svelte';
	import Badge from '$lib/components/ui/Badge.svelte';
	import Stars from '$lib/components/ui/Stars.svelte';
	import Spinner from '$lib/components/ui/Spinner.svelte';
	import { game } from '$lib/stores/game.svelte';
	import {
		fetchPlayerHistory,
		fetchPlayerStats,
		type MatchSummary,
		type PlayerStats
	} from '$lib/api/history';

	type FilterType = 'all' | 'won' | 'lost' | 'unrated';

	let loading = $state(true);
	let error = $state<string | null>(null);
	let matches = $state<MatchSummary[]>([]);
	let stats = $state<PlayerStats | null>(null);
	let activeFilter = $state<FilterType>('all');

	const effectivePlayerId = $derived(
		game.playerId ||
			(typeof localStorage !== 'undefined'
				? localStorage.getItem('proofbattle.player_id')
				: null) ||
			'00000000-0000-0000-0000-000000000001'
	);

	onMount(async () => {
		try {
			loading = true;
			error = null;
			const [historyData, statsData] = await Promise.all([
				fetchPlayerHistory(effectivePlayerId, 50).catch(() => []),
				fetchPlayerStats(effectivePlayerId).catch(() => null)
			]);

			matches = historyData;
			stats = statsData;
		} catch (e: unknown) {
			console.error('Failed to load history:', e);
			error = e instanceof Error ? e.message : 'Could not load match history';
		} finally {
			loading = false;
		}
	});

	const filteredMatches = $derived.by(() => {
		switch (activeFilter) {
			case 'won':
				return matches.filter((m) => m.outcome === 'Won' || m.outcome === 'ForfeitWin');
			case 'lost':
				return matches.filter((m) => m.outcome === 'Lost' || m.outcome === 'ForfeitLoss');
			case 'unrated':
				return matches.filter((m) => !m.rated);
			case 'all':
			default:
				return matches;
		}
	});

	function formatDuration(ms: number): string {
		const totalSecs = Math.round(ms / 1000);
		const mins = Math.floor(totalSecs / 60);
		const secs = totalSecs % 60;
		return mins > 0 ? `${mins}m ${secs}s` : `${secs}s`;
	}

	function formatDate(iso: string): string {
		try {
			const d = new Date(iso);
			return d.toLocaleDateString(undefined, {
				month: 'short',
				day: 'numeric',
				hour: '2-digit',
				minute: '2-digit'
			});
		} catch {
			return iso;
		}
	}

	function getOutcomeVariant(outcome: string): 'success' | 'error' | 'warning' | 'default' {
		switch (outcome) {
			case 'Won':
			case 'ForfeitWin':
				return 'success';
			case 'Lost':
			case 'ForfeitLoss':
				return 'error';
			case 'Draw':
				return 'warning';
			default:
				return 'default';
		}
	}
</script>

<svelte:head>
	<title>Match History: ProofBattle</title>
</svelte:head>

<div class="min-h-screen bg-[var(--bg-base)] px-4 py-8 sm:px-6 lg:px-8">
	<div class="mx-auto max-w-5xl space-y-8">
		<!-- Header Navigation Bar -->
		<div class="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
			<div>
				<h1 class="text-3xl font-black tracking-tight text-[var(--text-primary)]">
					Match History & Analytics
				</h1>
				<p class="mt-1 text-sm text-[var(--text-secondary)]">
					Review your past Lean theorem proving battles, analyze solve times, and study winning
					proofs.
				</p>
			</div>
			<div class="flex items-center gap-3">
				<Button variant="secondary" size="sm" onclick={() => goto('/lobby')}>Arena Lobby</Button>
				<Button variant="primary" size="sm" onclick={() => goto('/lobby')}>Find Match</Button>
			</div>
		</div>

		{#if loading}
			<div class="flex h-64 items-center justify-center">
				<Spinner size={32} />
			</div>
		{:else if error}
			<div
				class="rounded-[var(--radius-md)] border border-[var(--error)] bg-[rgba(248,113,113,0.1)] p-4 text-center text-sm text-[var(--error)]"
			>
				Failed to load history: {error}
			</div>
		{:else}
			<!-- Performance Overview Grid -->
			<div class="grid grid-cols-2 gap-4 sm:grid-cols-4">
				<div class="panel-elevated p-4 text-center">
					<span
						class="block text-xs font-semibold tracking-wider text-[var(--text-muted)] uppercase"
					>
						Current Elo
					</span>
					<span class="mt-1 block font-mono text-2xl font-bold text-[var(--accent-bright)]">
						{stats?.current_rating || game.elo || 1200}
					</span>
				</div>

				<div class="panel-elevated p-4 text-center">
					<span
						class="block text-xs font-semibold tracking-wider text-[var(--text-muted)] uppercase"
					>
						Games Played
					</span>
					<span class="mt-1 block font-mono text-2xl font-bold text-[var(--text-primary)]">
						{stats?.games_played || matches.length}
					</span>
				</div>

				<div class="panel-elevated p-4 text-center">
					<span
						class="block text-xs font-semibold tracking-wider text-[var(--text-muted)] uppercase"
					>
						Win Rate
					</span>
					<span class="mt-1 block font-mono text-2xl font-bold text-[var(--success)]">
						{stats ? `${Math.round(stats.win_rate)}%` : '0%'}
					</span>
				</div>

				<div class="panel-elevated p-4 text-center">
					<span
						class="block text-xs font-semibold tracking-wider text-[var(--text-muted)] uppercase"
					>
						Record (W/L/D)
					</span>
					<span class="mt-1 block font-mono text-lg font-bold text-[var(--text-secondary)]">
						{stats?.wins || 0}W / {stats?.losses || 0}L / {stats?.draws || 0}D
					</span>
				</div>
			</div>

			<!-- Mathematical Category Mastery -->
			{#if stats && stats.categories.length > 0}
				<div class="panel p-5">
					<h3 class="mb-3 text-xs font-bold tracking-wider text-[var(--text-muted)] uppercase">
						Category Performance
					</h3>
					<div class="grid grid-cols-1 gap-3 sm:grid-cols-3">
						{#each stats.categories as cat (cat.category)}
							<div class="panel-elevated p-3">
								<div class="flex items-center justify-between">
									<Badge variant="accent" size="sm">{cat.category}</Badge>
									<span class="font-mono text-xs font-bold text-[var(--success)]">
										{Math.round(cat.win_rate)}% Win
									</span>
								</div>
								<div class="mt-2 text-xs text-[var(--text-secondary)]">
									{cat.wins} wins in {cat.games_played} matches
								</div>
							</div>
						{/each}
					</div>
				</div>
			{/if}

			<!-- Filter Navigation Tabs -->
			<div class="flex items-center justify-between border-b border-[var(--bg-border)] pb-3">
				<div class="flex items-center gap-2">
					<button
						type="button"
						onclick={() => (activeFilter = 'all')}
						class="rounded-[var(--radius-sm)] px-3 py-1.5 text-xs font-bold transition-colors {activeFilter ===
						'all'
							? 'bg-[var(--accent)] text-white'
							: 'text-[var(--text-secondary)] hover:bg-[var(--bg-elevated)]'}"
					>
						All Matches ({matches.length})
					</button>
					<button
						type="button"
						onclick={() => (activeFilter = 'won')}
						class="rounded-[var(--radius-sm)] px-3 py-1.5 text-xs font-bold transition-colors {activeFilter ===
						'won'
							? 'bg-[var(--accent)] text-white'
							: 'text-[var(--text-secondary)] hover:bg-[var(--bg-elevated)]'}"
					>
						Victories
					</button>
					<button
						type="button"
						onclick={() => (activeFilter = 'lost')}
						class="rounded-[var(--radius-sm)] px-3 py-1.5 text-xs font-bold transition-colors {activeFilter ===
						'lost'
							? 'bg-[var(--accent)] text-white'
							: 'text-[var(--text-secondary)] hover:bg-[var(--bg-elevated)]'}"
					>
						Defeats
					</button>
					<button
						type="button"
						onclick={() => (activeFilter = 'unrated')}
						class="rounded-[var(--radius-sm)] px-3 py-1.5 text-xs font-bold transition-colors {activeFilter ===
						'unrated'
							? 'bg-[var(--accent)] text-white'
							: 'text-[var(--text-secondary)] hover:bg-[var(--bg-elevated)]'}"
					>
						Unrated / Private
					</button>
				</div>
			</div>

			<!-- Matches List -->
			{#if filteredMatches.length === 0}
				<div class="panel py-16 text-center">
					<p class="text-sm font-semibold text-[var(--text-primary)]">
						No matches found in this view.
					</p>
					<p class="mt-1 text-xs text-[var(--text-muted)]">
						Queue up for a match or create a private room to start proving.
					</p>
					<div class="mt-4">
						<Button variant="primary" size="md" onclick={() => goto('/lobby')}>Go to Lobby</Button>
					</div>
				</div>
			{:else}
				<div class="space-y-3">
					{#each filteredMatches as m (m.id)}
						<div
							class="panel-elevated flex flex-col justify-between gap-4 p-4 transition-colors hover:border-[var(--accent)] sm:flex-row sm:items-center"
						>
							<!-- Left Column: Outcome and Problem Goal -->
							<div class="flex items-start gap-3">
								<Badge variant={getOutcomeVariant(m.outcome)} size="md" class="mt-0.5 shrink-0">
									{m.outcome}
								</Badge>
								<div>
									<div class="flex items-center gap-2">
										<Badge variant="default" size="sm">{m.problem_category}</Badge>
										<Stars difficulty={m.problem_difficulty} />
										<span class="font-mono text-xs text-[var(--text-muted)]">
											{formatDate(m.created_at)}
										</span>
									</div>
									<div class="mt-1.5 font-mono text-sm font-bold text-[var(--text-primary)]">
										{m.problem_goal}
									</div>
									<div class="mt-1 text-xs text-[var(--text-secondary)]">
										vs <strong>{m.opponent_username || 'Opponent'}</strong> ({m.opponent_elo})
									</div>
								</div>
							</div>

							<!-- Right Column: Rating Change & Replay Action -->
							<div class="flex items-center justify-between gap-4 sm:justify-end">
								<div class="text-right">
									{#if m.rated}
										<div
											class="font-mono text-sm font-bold {m.elo_delta >= 0
												? 'text-[var(--success)]'
												: 'text-[var(--error)]'}"
										>
											{m.elo_delta >= 0 ? `+${m.elo_delta}` : m.elo_delta}
										</div>
									{:else}
										<span class="font-mono text-xs text-[var(--accent-bright)]">Unrated</span>
									{/if}
									<span class="block font-mono text-[11px] text-[var(--text-muted)]">
										{formatDuration(m.duration_ms)}
									</span>
								</div>

								<Button variant="secondary" size="sm" onclick={() => goto(`/replay/${m.id}`)}>
									Review Proof
								</Button>
							</div>
						</div>
					{/each}
				</div>
			{/if}
		{/if}
	</div>
</div>
