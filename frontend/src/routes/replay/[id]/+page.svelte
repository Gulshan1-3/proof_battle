<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import Button from '$lib/components/ui/Button.svelte';
	import Badge from '$lib/components/ui/Badge.svelte';
	import Stars from '$lib/components/ui/Stars.svelte';
	import Spinner from '$lib/components/ui/Spinner.svelte';
	import { fetchMatchDetail, type MatchDetail } from '$lib/api/history';

	let loading = $state(true);
	let error = $state<string | null>(null);
	let match = $state<MatchDetail | null>(null);
	let copiedWinning = $state(false);
	let copiedCanonical = $state(false);

	const matchId = $derived(page.params.id);

	onMount(async () => {
		if (!matchId) {
			error = 'No match ID specified';
			loading = false;
			return;
		}

		try {
			loading = true;
			error = null;
			match = await fetchMatchDetail(matchId);
			if (!match) {
				error = 'Match record not found';
			}
		} catch (e: unknown) {
			console.error('Failed to load match detail:', e);
			error = e instanceof Error ? e.message : 'Failed to load match record';
		} finally {
			loading = false;
		}
	});

	function handleCopyWinning() {
		if (!match?.winning_proof || typeof navigator === 'undefined') return;
		navigator.clipboard.writeText(match.winning_proof);
		copiedWinning = true;
		setTimeout(() => {
			copiedWinning = false;
		}, 2000);
	}

	function handleCopyCanonical() {
		if (!match?.canonical_proof || typeof navigator === 'undefined') return;
		navigator.clipboard.writeText(match.canonical_proof);
		copiedCanonical = true;
		setTimeout(() => {
			copiedCanonical = false;
		}, 2000);
	}

	function formatDuration(ms: number): string {
		const totalSecs = Math.round(ms / 1000);
		const mins = Math.floor(totalSecs / 60);
		const secs = totalSecs % 60;
		return mins > 0 ? `${mins}m ${secs}s` : `${secs}s`;
	}

	function formatDate(iso: string): string {
		try {
			const d = new Date(iso);
			return d.toLocaleString(undefined, {
				dateStyle: 'medium',
				timeStyle: 'short'
			});
		} catch {
			return iso;
		}
	}
</script>

<svelte:head>
	<title>Proof Replay: ProofBattle</title>
</svelte:head>

<div class="min-h-screen bg-[var(--bg-base)] px-4 py-8 sm:px-6 lg:px-8">
	<div class="mx-auto max-w-5xl space-y-6">
		<!-- Top Action Bar -->
		<div class="flex items-center justify-between border-b border-[var(--bg-border)] pb-4">
			<div class="flex items-center gap-3">
				<Button variant="secondary" size="sm" onclick={() => goto('/history')}>
					Back to History
				</Button>
				<span class="font-mono text-xs text-[var(--text-muted)]">
					Match ID: <strong class="text-[var(--text-primary)]">{matchId}</strong>
				</span>
			</div>

			<Button variant="primary" size="sm" onclick={() => goto('/lobby')}>Play Again</Button>
		</div>

		{#if loading}
			<div class="flex h-64 items-center justify-center">
				<Spinner size={32} />
			</div>
		{:else if error || !match}
			<div class="panel py-16 text-center">
				<p class="text-sm font-semibold text-[var(--error)]">
					{error || 'Match record not found.'}
				</p>
				<p class="mt-1 text-xs text-[var(--text-muted)]">
					This match might have been played in another session or removed.
				</p>
				<div class="mt-4">
					<Button variant="secondary" size="md" onclick={() => goto('/history')}>
						Return to Match History
					</Button>
				</div>
			</div>
		{:else}
			<!-- Match Overview Card -->
			<div
				class="panel-elevated flex flex-col justify-between gap-4 p-6 sm:flex-row sm:items-center"
			>
				<div>
					<div class="flex items-center gap-2">
						<Badge variant="accent">{match.problem_category}</Badge>
						<Stars difficulty={match.problem_difficulty} />
						<span class="font-mono text-xs text-[var(--text-muted)]">
							{formatDate(match.created_at)}
						</span>
					</div>
					<h2 class="mt-2 text-2xl font-black tracking-tight text-[var(--text-primary)]">
						{match.player1_username || 'Player 1'} vs {match.player2_username || 'Player 2'}
					</h2>
					<p class="mt-1 text-xs text-[var(--text-secondary)]">
						Outcome: <strong>{match.outcome}</strong> (Duration: {formatDuration(
							match.duration_ms
						)})
					</p>
				</div>

				<div class="flex items-center gap-4">
					<div
						class="rounded-lg border border-[var(--bg-border)] bg-[var(--bg-surface)] p-3 text-center"
					>
						<span class="block text-[10px] font-bold text-[var(--text-muted)] uppercase">
							{match.player1_username || 'Player 1'}
						</span>
						<span class="font-mono text-sm font-bold text-[var(--text-primary)]">
							{match.player1_elo_after}
						</span>
						{#if match.rated}
							<span
								class="block font-mono text-xs font-bold {match.elo_delta_p1 >= 0
									? 'text-[var(--success)]'
									: 'text-[var(--error)]'}"
							>
								{match.elo_delta_p1 >= 0 ? `+${match.elo_delta_p1}` : match.elo_delta_p1}
							</span>
						{/if}
					</div>

					<div class="font-mono text-xs font-bold text-[var(--text-muted)]">vs</div>

					<div
						class="rounded-lg border border-[var(--bg-border)] bg-[var(--bg-surface)] p-3 text-center"
					>
						<span class="block text-[10px] font-bold text-[var(--text-muted)] uppercase">
							{match.player2_username || 'Player 2'}
						</span>
						<span class="font-mono text-sm font-bold text-[var(--text-primary)]">
							{match.player2_elo_after}
						</span>
						{#if match.rated}
							<span
								class="block font-mono text-xs font-bold {match.elo_delta_p2 >= 0
									? 'text-[var(--success)]'
									: 'text-[var(--error)]'}"
							>
								{match.elo_delta_p2 >= 0 ? `+${match.elo_delta_p2}` : match.elo_delta_p2}
							</span>
						{/if}
					</div>
				</div>
			</div>

			<!-- Goal to Prove Panel -->
			<div class="panel p-5">
				<span
					class="mb-2 block text-xs font-bold tracking-wider text-[var(--text-muted)] uppercase"
				>
					Goal to Prove
				</span>
				<div
					class="panel-elevated p-3 font-mono text-base font-bold break-words text-[var(--text-primary)]"
				>
					{match.problem_goal}
				</div>
			</div>

			<!-- Proof Comparison Grid -->
			<div class="grid grid-cols-1 gap-6 {match.canonical_proof ? 'lg:grid-cols-2' : ''}">
				<!-- Winning Proof Section -->
				<div class="panel flex flex-col p-5">
					<div class="mb-3 flex items-center justify-between">
						<span class="text-xs font-bold tracking-wider text-[var(--success)] uppercase">
							Winning Proof Script
						</span>
						{#if match.winning_proof}
							<Button variant="secondary" size="sm" onclick={handleCopyWinning}>
								{copiedWinning ? 'Copied' : 'Copy Proof'}
							</Button>
						{/if}
					</div>

					{#if match.winning_proof}
						<pre
							class="panel-elevated max-h-96 flex-1 overflow-auto p-4 font-mono text-xs leading-relaxed text-[var(--success)] select-all"><code
								>{match.winning_proof}</code
							></pre>
					{:else}
						<div
							class="panel-elevated flex h-32 items-center justify-center p-4 text-center font-mono text-xs text-[var(--text-muted)]"
						>
							No winning proof recorded (round ended via forfeit or timeout).
						</div>
					{/if}
				</div>

				<!-- Canonical Solution Section -->
				{#if match.canonical_proof}
					<div class="panel flex flex-col p-5">
						<div class="mb-3 flex items-center justify-between">
							<span class="text-xs font-bold tracking-wider text-[var(--accent-bright)] uppercase">
								Canonical Mathlib Solution
							</span>
							<Button variant="secondary" size="sm" onclick={handleCopyCanonical}>
								{copiedCanonical ? 'Copied' : 'Copy Solution'}
							</Button>
						</div>

						<pre
							class="panel-elevated max-h-96 flex-1 overflow-auto p-4 font-mono text-xs leading-relaxed text-[var(--text-primary)] select-all"><code
								>{match.canonical_proof}</code
							></pre>
					</div>
				{/if}
			</div>
		{/if}
	</div>
</div>
