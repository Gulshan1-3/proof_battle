<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import Button from '$lib/components/ui/Button.svelte';
	import Badge from '$lib/components/ui/Badge.svelte';
	import Stars from '$lib/components/ui/Stars.svelte';
	import Timer from '$lib/components/ui/Timer.svelte';
	import ResultModal from '$lib/components/game/ResultModal.svelte';
	import DiagnosticsBar from '$lib/components/game/DiagnosticsBar.svelte';
	import { Editor, UnicodePalette } from '$lib/editor';
	import { game } from '$lib/stores/game.svelte';
	import { wsClient } from '$lib/ws/client';
	import { MockWebSocket } from '$lib/mock/server';

	let tacticCode = $state('intro n\nsimp');
	let showHint = $state(false);
	let showResultModal = $state(false);
	interface EditorMethods {
		getValue(): string;
		focus(): void;
		insertAtCursor(text: string): void;
		setPosition(lineNumber: number, column?: number): void;
	}

	let editorRef = $state<EditorMethods | null>(null);
	let debounceTimer: ReturnType<typeof setTimeout> | null = null;

	function handleCodeChange(newCode: string) {
		tacticCode = newCode;
		game.clearSubmitDiagnostics();

		if (debounceTimer) clearTimeout(debounceTimer);
		debounceTimer = setTimeout(() => {
			if (game.canCheck && wsClient.state === 'in_game') {
				wsClient.checkProof(newCode).catch(() => {});
			}
		}, 800);
	}

	onDestroy(() => {
		if (debounceTimer) {
			clearTimeout(debounceTimer);
		}
	});

	onMount(() => {
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
	});

	// Generated Lean wrapper preview
	const composedLeanFile = $derived.by(() => {
		const imports = game.problem?.imports.join('\n') || 'import Mathlib.Data.Nat.Basic';
		const goal = game.problem?.goal || '∀ n : ℕ, n + 0 = n';
		return `${imports}\n\ntheorem proofbattle_goal : ${goal} := by\n  ${tacticCode.split('\n').join('\n  ')}`;
	});

	$effect(() => {
		if (game.result) {
			showResultModal = true;
		}
	});

	async function handleCheck() {
		if (!game.canCheck) return;
		try {
			await wsClient.checkProof(tacticCode);
		} catch (e) {
			console.error('Check proof failed:', e);
		}
	}

	async function handleSubmit() {
		if (!game.canSubmit) return;
		try {
			await wsClient.submitProof(tacticCode);
		} catch (e) {
			console.error('Submit proof failed:', e);
		}
	}

	function handleResign() {
		if (confirm('Are you sure you want to forfeit this match?')) {
			wsClient.resign();
		}
	}

	const isPractice = $derived(
		typeof window !== 'undefined' &&
		(new URLSearchParams(window.location.search).get('practice') === '1' ||
			game.opponent?.username?.toLowerCase().includes('bot'))
	);

	function handleRematch() {
		showResultModal = false;
		if (isPractice) {
			wsClient.practiceJoin();
		} else {
			wsClient.joinQueue();
		}
		const isMock =
			typeof sessionStorage !== 'undefined' && sessionStorage.getItem('proofbattle.mock') === '1';
		const params = new URLSearchParams();
		if (isMock) params.set('mock', '1');
		if (isPractice) params.set('practice', '1');
		const query = params.toString() ? '?' + params.toString() : '';
		goto('/lobby' + query);
	}

	function handleNewMatch() {
		showResultModal = false;
		wsClient.leaveQueue();
		goto('/lobby');
	}
</script>

<div class="flex h-screen flex-1 flex-col overflow-hidden bg-[var(--bg-base)]">
	<!-- Competitive 1v1 Battle Header (Chess.com inspired) -->
	<header
		class="flex h-14 shrink-0 items-center justify-between border-b border-[var(--bg-border)] bg-[var(--bg-surface)] px-6"
	>
		<!-- Player Card Left -->
		<div class="flex items-center gap-3">
			<div
				class="flex h-8 w-8 items-center justify-center rounded-[var(--radius-sm)] bg-[var(--accent)] font-mono text-xs font-bold text-white"
			>
				YOU
			</div>
			<div>
				<div class="flex items-center gap-2">
					<span class="text-xs font-bold text-[var(--text-primary)]">
						{game.you?.username || game.username || 'You'}
					</span>
					<span class="font-mono text-[11px] text-[var(--text-secondary)]"
						>({game.you?.elo || game.elo})</span
					>
				</div>
				<span class="flex items-center gap-1 text-[10px] font-medium text-[var(--success)]">
					<span class="h-1.5 w-1.5 rounded-full bg-[var(--success)]"></span> Active
				</span>
			</div>
		</div>

		<!-- Match Countdown Clock -->
		<div class="flex items-center gap-2">
			<Timer
				endsAtMs={game.endsAtMs}
				serverTimeOffsetMs={game.serverTimeOffsetMs}
				active={wsClient.state === 'in_game' || wsClient.state === 'submitting'}
			/>
		</div>

		<!-- Opponent Card Right -->
		<div class="flex items-center gap-3">
			<div class="text-right">
				<div class="flex items-center justify-end gap-2">
					<span class="font-mono text-[11px] text-[var(--text-secondary)]"
						>({game.opponent?.elo || 1240})</span
					>
					<span class="text-xs font-bold text-[var(--text-primary)]">
						{game.opponent?.username || 'Opponent'}
					</span>
				</div>
				<span class="text-[10px] font-medium text-[var(--accent-bright)]">
					{game.opponentStatus}
				</span>
			</div>
			<div
				class="flex h-8 w-8 items-center justify-center rounded-[var(--radius-sm)] border border-[var(--bg-border)] bg-[var(--bg-elevated)] font-mono text-xs font-bold text-[var(--text-secondary)]"
			>
				{isPractice || game.opponent?.username?.toLowerCase().includes('bot') ? 'BOT' : 'OPP'}
			</div>
		</div>
	</header>

	<!-- 3-Panel Competitive Arena -->
	<main class="grid flex-1 grid-cols-1 overflow-hidden md:grid-cols-[300px_1fr_260px]">
		<!-- Left: Theorem Specification Panel -->
		<aside
			class="panel flex flex-col justify-between overflow-y-auto border-r border-[var(--bg-border)] bg-[var(--bg-surface)] p-5"
		>
			<div class="space-y-4">
				<div class="flex items-center justify-between">
					<Badge variant="accent">{game.problem?.category || 'Logic'}</Badge>
					<Stars difficulty={game.problem?.difficulty || 1} />
				</div>

				<div>
					<span
						class="mb-1 block text-xs font-semibold tracking-wider text-[var(--text-secondary)] uppercase"
					>
						Goal to Prove:
					</span>
					<div
						class="panel-elevated p-3 font-mono text-sm font-bold break-words text-[var(--text-primary)]"
					>
						{game.problem?.goal || '∀ n : ℕ, n + 0 = n'}
					</div>
				</div>

				<div>
					<span
						class="mb-1 block text-xs font-semibold tracking-wider text-[var(--text-secondary)] uppercase"
					>
						Required Imports:
					</span>
					<ul class="space-y-1 font-mono text-xs text-[var(--text-secondary)]">
						{#each game.problem?.imports || ['import Mathlib.Data.Nat.Basic'] as imp (imp)}
							<li class="panel-elevated px-2 py-1">{imp}</li>
						{/each}
					</ul>
				</div>

				{#if game.problem?.hint}
					<div class="pt-2">
						<button
							type="button"
							onclick={() => (showHint = !showHint)}
							class="flex cursor-pointer items-center gap-1.5 text-xs font-semibold text-[var(--accent-bright)] hover:underline"
						>
							<svg
								width="14"
								height="14"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="2"
							>
								<circle cx="12" cy="12" r="10"></circle>
								<line x1="12" y1="16" x2="12" y2="12"></line>
								<line x1="12" y1="8" x2="12.01" y2="8"></line>
							</svg>
							{showHint ? 'Hide Tactical Hint' : 'Show Tactical Hint'}
						</button>
						{#if showHint}
							<div
								class="panel-elevated mt-2 border-l-2 border-[var(--accent)] p-2.5 text-xs text-[var(--text-secondary)]"
							>
								{game.problem.hint}
							</div>
						{/if}
					</div>
				{/if}
			</div>

			<div class="border-t border-[var(--bg-border)] pt-4">
				<Button variant="danger" size="sm" onclick={handleResign} class="w-full">
					Resign Match
				</Button>
			</div>
		</aside>

		<!-- Center: Editor Workspace & Live Diagnostics (Stage S8 Monaco Integration) -->
		<section class="flex flex-1 flex-col overflow-hidden bg-[var(--editor-bg)]">
			<!-- Editor Top Bar -->
			<div
				class="flex h-10 shrink-0 items-center justify-between border-b border-[var(--bg-border)] bg-[var(--bg-surface)] px-4 font-mono text-xs text-[var(--text-secondary)]"
			>
				<div class="flex items-center gap-2">
					<span class="font-bold text-[var(--text-primary)]">solution.lean</span>
					<span class="text-[var(--text-muted)]">(Tactic Body Workspace)</span>
				</div>
				<div class="flex items-center gap-3">
					{#if game.isChecking}
						<span class="flex items-center gap-1.5 text-xs text-[var(--accent-bright)]">
							<span class="inline-block h-2 w-2 animate-pulse rounded-full bg-[var(--accent)]"
							></span>
							Checking…
						</span>
					{/if}
					<span class="text-[var(--text-muted)]">Lean 4.21.0-rc3</span>
				</div>
			</div>

			<!-- Lean 4 Unicode Symbol Palette -->
			<UnicodePalette oninsert={(symbol) => editorRef?.insertAtCursor(symbol)} />

			<!-- Monaco Editor Container -->
			<div class="relative min-h-[280px] flex-1 overflow-hidden">
				<Editor
					bind:this={editorRef}
					bind:value={tacticCode}
					diagnostics={game.activeDiagnostics}
					onchange={handleCodeChange}
				/>
			</div>

			<!-- Compiler Diagnostics Bar (Click to jump into Monaco) -->
			<DiagnosticsBar
				diagnostics={game.activeDiagnostics}
				checking={game.isChecking}
				onjump={(line, col) => editorRef?.setPosition(line, col)}
			/>

			<!-- Collapsible Authoritative Server Wrapper Drawer -->
			<details
				class="group border-t border-[var(--bg-border)] bg-[var(--bg-surface)] text-xs text-[var(--text-secondary)]"
			>
				<summary
					class="flex cursor-pointer items-center justify-between px-4 py-1.5 font-mono text-[11px] text-[var(--text-muted)] hover:text-[var(--text-primary)]"
				>
					<span>Authoritative Server Wrapper Preview:</span>
					<span class="text-[10px] text-[var(--accent-bright)] group-open:hidden"
						>Show Full .lean Code ▼</span
					>
					<span class="hidden text-[10px] text-[var(--accent-bright)] group-open:inline"
						>Hide ▲</span
					>
				</summary>
				<pre
					class="max-h-36 overflow-y-auto border-t border-[var(--bg-border)] bg-[var(--bg-base)] p-3 font-mono text-xs text-[var(--text-secondary)] select-all"><code
						>{composedLeanFile}</code
					></pre>
			</details>

			<!-- Submit & Check Actions Toolbar -->
			<div
				class="flex h-14 shrink-0 items-center justify-between border-t border-[var(--bg-border)] bg-[var(--bg-surface)] px-4"
			>
				<div class="font-mono text-xs text-[var(--text-muted)]">
					<span
						>Shortcuts: <kbd class="rounded bg-[var(--bg-overlay)] px-1 py-0.5">Ctrl+Enter</kbd> submit</span
					>
					<span class="ml-2">Submit limit: 5 / 60s</span>
				</div>

				<div class="flex items-center gap-3">
					<Button
						variant="secondary"
						size="md"
						loading={game.isChecking}
						disabled={!game.canCheck}
						onclick={handleCheck}
					>
						Check Only
					</Button>

					<Button
						variant="primary"
						size="md"
						loading={game.isSubmitting}
						disabled={!game.canSubmit}
						onclick={handleSubmit}
					>
						Submit Proof
					</Button>
				</div>
			</div>
		</section>

		<!-- Right: Telemetry & Compiler Diagnostics Panel -->
		<aside
			class="panel flex flex-col overflow-y-auto border-l border-[var(--bg-border)] bg-[var(--bg-surface)] p-4"
		>
			<div class="mb-3 flex items-center justify-between border-b border-[var(--bg-border)] pb-3">
				<span class="text-xs font-bold tracking-wider text-[var(--text-secondary)] uppercase">
					Compiler Feedback
				</span>
				{#if game.lastVerdict}
					<Badge variant={game.lastVerdict.verdict === 'Accepted' ? 'success' : 'error'} size="sm">
						{game.lastVerdict.verdict}
					</Badge>
				{/if}
			</div>

			<div class="flex-1 space-y-3">
				{#if game.lastVerdict}
					<div class="panel-elevated space-y-1 p-3 font-mono text-xs">
						<div class="font-bold text-[var(--text-primary)]">{game.lastVerdict.message}</div>
						<div class="text-[var(--text-muted)]">
							Elapsed: {game.lastVerdict.elapsedMs.toString()} ms
						</div>
						{#if game.checkSkipped}
							<div class="text-[var(--warning)]">Notice: Diagnostic check skipped due to load.</div>
						{/if}
					</div>
				{/if}

				{#if game.diagnostics.length > 0}
					<div class="space-y-2">
						<span class="block text-[11px] font-bold tracking-wider text-[var(--error)] uppercase">
							Diagnostics ({game.diagnostics.length})
						</span>
						{#each game.diagnostics as diag, i (`${diag.line}:${diag.col}:${i}`)}
							<div
								class="panel-elevated border-l-2 border-[var(--error)] p-2 font-mono text-xs text-[var(--error)]"
							>
								<span class="text-[var(--text-muted)]">L{diag.line}:C{diag.col}</span> — {diag.message}
							</div>
						{/each}
					</div>
				{:else if !game.lastVerdict}
					<div class="py-8 text-center text-xs text-[var(--text-muted)]">
						No diagnostic messages. Run "Check Only" or "Submit Proof" to inspect elaboration.
					</div>
				{/if}
			</div>
		</aside>
	</main>
</div>

{#if game.result}
	<ResultModal
		bind:open={showResultModal}
		result={game.result}
		onrematch={handleRematch}
		onnewmatch={handleNewMatch}
	/>
{/if}
