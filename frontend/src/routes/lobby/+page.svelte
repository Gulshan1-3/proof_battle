<script lang="ts">
	import { goto } from '$app/navigation';
	import Button from '$lib/components/ui/Button.svelte';
	import Badge from '$lib/components/ui/Badge.svelte';
	import { game } from '$lib/stores/game.svelte';
	import { wsClient } from '$lib/ws/client';
	import { MockWebSocket } from '$lib/mock/server';

	type LobbyMode = 'ranked' | 'private' | 'practice';
	type PrivateTab = 'create' | 'join' | 'spectate';

	let mode = $state<LobbyMode>('ranked');
	let privateTab = $state<PrivateTab>('create');

	// Custom room settings
	let customCategory = $state<string>('');
	let customDifficulty = $state<number>(0);
	let customDurationSecs = $state<number>(300);

	let joinCode = $state('');
	let spectateCode = $state('');
	let copiedLink = $state(false);
	let copiedCode = $state(false);
	let elapsedSecs = $state(0);

	$effect(() => {
		if (typeof window === 'undefined') return;

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

		if (params.get('practice') === '1') {
			mode = 'practice';
			triggerModeAction('practice');
		} else if (params.get('join')) {
			mode = 'private';
			privateTab = 'join';
			joinCode = params.get('join')!.toUpperCase();
		} else if (params.get('spectate')) {
			mode = 'private';
			privateTab = 'spectate';
			spectateCode = params.get('spectate')!.toUpperCase();
		} else if (params.get('private') === '1') {
			mode = 'private';
			privateTab = 'create';
		} else {
			mode = 'ranked';
			triggerModeAction('ranked');
		}

		const timer = setInterval(() => {
			elapsedSecs += 1;
		}, 1000);

		return () => {
			clearInterval(timer);
		};
	});

	function triggerModeAction(targetMode: LobbyMode) {
		if (targetMode === 'ranked') {
			const check = setInterval(() => {
				if (wsClient.state === 'connected_idle' || wsClient.state === 'game_ended') {
					wsClient.joinQueue();
					clearInterval(check);
				}
			}, 100);
		} else if (targetMode === 'practice') {
			const check = setInterval(() => {
				if (wsClient.state === 'connected_idle' || wsClient.state === 'game_ended') {
					wsClient.practiceJoin();
					clearInterval(check);
				}
			}, 100);
		}
	}

	function handleModeSelect(newMode: LobbyMode) {
		mode = newMode;
		elapsedSecs = 0;
		game.serverError = null;
		wsClient.leaveQueue();

		if (newMode === 'ranked') {
			wsClient.joinQueue();
		} else if (newMode === 'practice') {
			wsClient.practiceJoin();
		}
	}

	function handleCreatePrivate() {
		game.serverError = null;
		const cat = customCategory.trim() ? customCategory.trim() : undefined;
		const diff = customDifficulty > 0 ? customDifficulty : undefined;
		wsClient.createPrivateRoom(cat, diff, customDurationSecs);
	}

	function handleJoinPrivate() {
		if (!joinCode.trim()) return;
		game.serverError = null;
		wsClient.joinPrivateRoom(joinCode.trim());
	}

	function handleSpectate() {
		if (!spectateCode.trim()) return;
		game.serverError = null;
		wsClient.spectateRoom(spectateCode.trim());
	}

	function handleCopyCode() {
		if (!game.privateRoomCode || typeof navigator === 'undefined') return;
		navigator.clipboard.writeText(game.privateRoomCode);
		copiedCode = true;
		setTimeout(() => {
			copiedCode = false;
		}, 2000);
	}

	function handleCopyInviteLink() {
		if (!game.privateRoomCode || typeof window === 'undefined') return;
		const link = `${window.location.origin}/lobby?join=${game.privateRoomCode}`;
		navigator.clipboard.writeText(link);
		copiedLink = true;
		setTimeout(() => {
			copiedLink = false;
		}, 2000);
	}

	// Reactive redirect when matched or spectating
	$effect(() => {
		if (
			game.roomId &&
			(wsClient.state === 'game_starting' ||
				wsClient.state === 'in_game' ||
				wsClient.state === 'spectating')
		) {
			const isMock =
				typeof sessionStorage !== 'undefined' && sessionStorage.getItem('proofbattle.mock') === '1';
			const queryParts: string[] = [];
			if (isMock) queryParts.push('mock=1');
			if (mode === 'practice') queryParts.push('practice=1');
			const query = queryParts.length > 0 ? '?' + queryParts.join('&') : '';
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
	class="mx-auto flex w-full max-w-2xl flex-1 flex-col items-center justify-center p-6 text-center"
>
	<!-- Top navigation bar -->
	<div class="mb-6 flex w-full items-center justify-between">
		<a
			href="/"
			class="flex items-center gap-1.5 text-xs font-semibold text-[var(--text-secondary)] transition-colors hover:text-[var(--text-primary)]"
		>
			<svg
				width="14"
				height="14"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
			>
				<path d="M19 12H5M12 19l-7-7 7-7" />
			</svg>
			Back to Arena
		</a>
		<a
			href="/history"
			class="rounded-[var(--radius-md)] border border-[var(--bg-border)] px-3 py-1 text-xs font-semibold text-[var(--text-secondary)] transition-colors hover:border-[var(--accent)] hover:text-[var(--text-primary)]"
		>
			Match History
		</a>
	</div>

	<!-- Mode Switcher Tabs -->
	<div
		class="mb-6 inline-flex rounded-[var(--radius-md)] border border-[var(--bg-border)] bg-[var(--bg-surface)] p-1 text-xs font-semibold"
	>
		<button
			type="button"
			onclick={() => handleModeSelect('ranked')}
			class="cursor-pointer rounded-[var(--radius-sm)] px-4 py-2 transition-all {mode === 'ranked'
				? 'bg-[var(--accent)] font-bold text-white shadow-sm'
				: 'text-[var(--text-secondary)] hover:text-[var(--text-primary)]'}"
		>
			Ranked Match
		</button>
		<button
			type="button"
			onclick={() => handleModeSelect('private')}
			class="cursor-pointer rounded-[var(--radius-sm)] px-4 py-2 transition-all {mode === 'private'
				? 'bg-[var(--accent)] font-bold text-white shadow-sm'
				: 'text-[var(--text-secondary)] hover:text-[var(--text-primary)]'}"
		>
			Private Duel
		</button>
		<button
			type="button"
			onclick={() => handleModeSelect('practice')}
			class="cursor-pointer rounded-[var(--radius-sm)] px-4 py-2 transition-all {mode === 'practice'
				? 'bg-[var(--accent)] font-bold text-white shadow-sm'
				: 'text-[var(--text-secondary)] hover:text-[var(--text-primary)]'}"
		>
			Solo Practice
		</button>
	</div>

	<!-- Ranked Mode View -->
	{#if mode === 'ranked'}
		<div class="relative mb-6 flex h-32 w-32 items-center justify-center">
			<div
				class="pulse absolute inset-0 rounded-full border border-[var(--accent)] opacity-20"
			></div>
			<div
				class="absolute inset-3 animate-ping rounded-full border border-[var(--accent)] opacity-40"
			></div>
			<div
				class="relative flex h-16 w-16 items-center justify-center rounded-full border-2 border-[var(--accent)] bg-[var(--bg-surface)] shadow-lg"
			>
				<svg
					width="28"
					height="28"
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

		<Badge variant="accent" class="mb-2">Ranked Arena</Badge>
		<h2 class="mb-1 text-2xl font-black tracking-tight text-[var(--text-primary)]">
			Finding Your Opponent...
		</h2>
		<p class="mb-6 font-mono text-sm text-[var(--text-secondary)]">
			Queue Time: {Math.floor(elapsedSecs / 60)}:{(elapsedSecs % 60).toString().padStart(2, '0')}
		</p>

		<div class="panel mb-6 w-full space-y-3 p-5 text-left">
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
				Connected to Judge Cluster: Searching
			</div>
		</div>

		<Button variant="secondary" size="md" onclick={handleCancel} class="w-full">
			Cancel Matchmaking
		</Button>

		<!-- Solo Practice Mode View -->
	{:else if mode === 'practice'}
		<div class="relative mb-6 flex h-32 w-32 items-center justify-center">
			<div
				class="pulse absolute inset-0 rounded-full border border-[var(--accent)] opacity-20"
			></div>
			<div
				class="relative flex h-16 w-16 items-center justify-center rounded-full border-2 border-[var(--accent)] bg-[var(--bg-surface)] shadow-lg"
			>
				<svg
					width="28"
					height="28"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
					class="text-[var(--accent-bright)]"
				>
					<rect x="3" y="11" width="18" height="10" rx="2"></rect>
					<circle cx="12" cy="5" r="2"></circle>
					<path d="M12 7v4"></path>
					<line x1="8" y1="16" x2="8" y2="16"></line>
					<line x1="16" y1="16" x2="16" y2="16"></line>
				</svg>
			</div>
		</div>

		<Badge variant="accent" class="mb-2">Solo Practice Arena</Badge>
		<h2 class="mb-1 text-2xl font-black tracking-tight text-[var(--text-primary)]">
			Preparing Your Sandbox Challenge...
		</h2>
		<p class="mb-6 font-mono text-sm text-[var(--text-secondary)]">
			Setup Time: {Math.floor(elapsedSecs / 60)}:{(elapsedSecs % 60).toString().padStart(2, '0')}
		</p>

		<div class="panel mb-6 w-full space-y-3 p-5 text-left">
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
		</div>

		<Button variant="secondary" size="md" onclick={handleCancel} class="w-full">
			Cancel Practice
		</Button>

		<!-- Private Duel Mode View -->
	{:else}
		<!-- Error banner if any -->
		{#if game.serverError}
			<div
				class="mb-4 flex w-full items-center justify-between rounded-[var(--radius-md)] border border-[var(--error)] bg-[rgba(248,113,113,0.1)] px-4 py-2.5 text-xs text-[var(--error)]"
			>
				<span><strong>Error:</strong> {game.serverError.message}</span>
				<button
					type="button"
					onclick={() => (game.serverError = null)}
					class="cursor-pointer font-bold text-[var(--error)] hover:opacity-80"
				>
					x
				</button>
			</div>
		{/if}

		{#if game.privateRoomCode}
			<!-- Waiting for friend to join -->
			<div class="relative mb-6 flex h-28 w-28 items-center justify-center">
				<div
					class="pulse absolute inset-0 rounded-full border border-[var(--accent)] opacity-20"
				></div>
				<div
					class="absolute inset-2 animate-ping rounded-full border border-[var(--accent)] opacity-40"
				></div>
				<div
					class="relative flex h-14 w-14 items-center justify-center rounded-full border-2 border-[var(--accent)] bg-[var(--bg-surface)] shadow-lg"
				>
					<svg
						width="24"
						height="24"
						viewBox="0 0 24 24"
						fill="none"
						stroke="var(--accent-bright)"
						stroke-width="2"
					>
						<path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"></path>
						<circle cx="9" cy="7" r="4"></circle>
						<path d="M23 21v-2a4 4 0 0 0-3-3.87"></path>
						<path d="M16 3.13a4 4 0 0 1 0 7.75"></path>
					</svg>
				</div>
			</div>

			<Badge variant="accent" class="mb-2">Private Room Ready</Badge>
			<h2 class="mb-1 text-2xl font-black tracking-tight text-[var(--text-primary)]">
				Share Room Code with Friend
			</h2>
			<p class="mb-5 text-xs text-[var(--text-secondary)]">
				Your opponent can join using this code or the direct invitation link.
			</p>

			<div class="panel mb-6 w-full space-y-4 p-5 text-center">
				<div class="flex flex-col items-center justify-center gap-1">
					<span
						class="text-[11px] font-semibold tracking-wider text-[var(--text-secondary)] uppercase"
					>
						Room Code
					</span>
					<div
						class="rounded-[var(--radius-md)] border-2 border-[var(--accent)] bg-[var(--bg-elevated)] px-6 py-2.5 font-mono text-3xl font-black tracking-widest text-[var(--accent-bright)]"
					>
						{game.privateRoomCode}
					</div>
				</div>

				<div class="flex items-center justify-center gap-3">
					<Button variant="secondary" size="sm" onclick={handleCopyCode}>
						{copiedCode ? 'Copied Code!' : 'Copy Code'}
					</Button>
					<Button variant="secondary" size="sm" onclick={handleCopyInviteLink}>
						{copiedLink ? 'Copied Link!' : 'Copy Invite Link'}
					</Button>
				</div>

				<div
					class="flex items-center justify-center gap-2 border-t border-[var(--bg-border)] pt-3 text-xs text-[var(--text-muted)]"
				>
					<span class="h-2 w-2 animate-pulse rounded-full bg-[var(--accent)]"></span>
					Waiting for challenger to enter room...
				</div>
			</div>

			<Button variant="danger" size="md" onclick={handleCancel} class="w-full">Cancel Duel</Button>
		{:else}
			<!-- Subtabs: Create, Join, Spectate -->
			<div
				class="mb-6 flex w-full border-b border-[var(--bg-border)] font-mono text-xs font-semibold"
			>
				<button
					type="button"
					onclick={() => (privateTab = 'create')}
					class="cursor-pointer border-b-2 px-4 py-2.5 transition-all {privateTab === 'create'
						? 'border-[var(--accent)] text-[var(--accent-bright)]'
						: 'border-transparent text-[var(--text-muted)] hover:text-[var(--text-primary)]'}"
				>
					Create Duel
				</button>
				<button
					type="button"
					onclick={() => (privateTab = 'join')}
					class="cursor-pointer border-b-2 px-4 py-2.5 transition-all {privateTab === 'join'
						? 'border-[var(--accent)] text-[var(--accent-bright)]'
						: 'border-transparent text-[var(--text-muted)] hover:text-[var(--text-primary)]'}"
				>
					Join Duel
				</button>
				<button
					type="button"
					onclick={() => (privateTab = 'spectate')}
					class="cursor-pointer border-b-2 px-4 py-2.5 transition-all {privateTab === 'spectate'
						? 'border-[var(--accent)] text-[var(--accent-bright)]'
						: 'border-transparent text-[var(--text-muted)] hover:text-[var(--text-primary)]'}"
				>
					Spectate Live
				</button>
			</div>

			{#if privateTab === 'create'}
				<!-- Create Duel Options Form -->
				<div class="panel mb-6 w-full space-y-4 p-5 text-left">
					<h3 class="text-sm font-bold text-[var(--text-primary)]">Custom Duel Settings</h3>

					<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
						<div>
							<label
								for="custom-category"
								class="mb-1 block text-xs font-semibold text-[var(--text-secondary)]"
							>
								Category
							</label>
							<select
								id="custom-category"
								bind:value={customCategory}
								class="w-full rounded-[var(--radius-sm)] border border-[var(--bg-border)] bg-[var(--bg-elevated)] p-2 font-mono text-xs text-[var(--text-primary)] focus:border-[var(--accent)] focus:outline-none"
							>
								<option value="">Any Category</option>
								<option value="logic">Propositional Logic</option>
								<option value="arithmetic">Nat Arithmetic</option>
								<option value="algebra">Algebra & Equality</option>
							</select>
						</div>

						<div>
							<label
								for="custom-difficulty"
								class="mb-1 block text-xs font-semibold text-[var(--text-secondary)]"
							>
								Difficulty
							</label>
							<select
								id="custom-difficulty"
								bind:value={customDifficulty}
								class="w-full rounded-[var(--radius-sm)] border border-[var(--bg-border)] bg-[var(--bg-elevated)] p-2 font-mono text-xs text-[var(--text-primary)] focus:border-[var(--accent)] focus:outline-none"
							>
								<option value={0}>Any Difficulty</option>
								<option value={1}>Tier 1: Introductory</option>
								<option value={2}>Tier 2: Intermediate</option>
								<option value={3}>Tier 3: Advanced</option>
							</select>
						</div>
					</div>

					<div>
						<label
							for="custom-duration"
							class="mb-1 block text-xs font-semibold text-[var(--text-secondary)]"
						>
							Round Duration
						</label>
						<select
							id="custom-duration"
							bind:value={customDurationSecs}
							class="w-full rounded-[var(--radius-sm)] border border-[var(--bg-border)] bg-[var(--bg-elevated)] p-2 font-mono text-xs text-[var(--text-primary)] focus:border-[var(--accent)] focus:outline-none"
						>
							<option value={60}>1 Minute Blitz</option>
							<option value={120}>2 Minutes Rapid</option>
							<option value={300}>5 Minutes Classical</option>
							<option value={600}>10 Minutes Marathon</option>
						</select>
					</div>

					<div class="border-t border-[var(--bg-border)] pt-2 text-[11px] text-[var(--text-muted)]">
						Notice: Private custom matches are unrated. No Elo rating is changed.
					</div>

					<Button variant="primary" size="md" onclick={handleCreatePrivate} class="w-full">
						Create Private Room
					</Button>
				</div>
			{:else if privateTab === 'join'}
				<!-- Join Duel Form -->
				<div class="panel mb-6 w-full space-y-4 p-5 text-left">
					<h3 class="text-sm font-bold text-[var(--text-primary)]">Enter Room Code</h3>
					<p class="text-xs text-[var(--text-secondary)]">
						Enter the 6-character room code given to you by the room host.
					</p>

					<div>
						<input
							type="text"
							bind:value={joinCode}
							maxlength="6"
							placeholder="e.g. MATH42"
							class="w-full rounded-[var(--radius-sm)] border border-[var(--bg-border)] bg-[var(--bg-elevated)] p-3 text-center font-mono text-xl font-bold tracking-widest text-[var(--text-primary)] uppercase focus:border-[var(--accent)] focus:outline-none"
						/>
					</div>

					<Button
						variant="primary"
						size="md"
						disabled={joinCode.trim().length < 6}
						onclick={handleJoinPrivate}
						class="w-full"
					>
						Join Match
					</Button>
				</div>
			{:else if privateTab === 'spectate'}
				<!-- Spectate Live Form -->
				<div class="panel mb-6 w-full space-y-4 p-5 text-left">
					<h3 class="text-sm font-bold text-[var(--text-primary)]">Watch Live Duel</h3>
					<p class="text-xs text-[var(--text-secondary)]">
						Enter an active Room Code or Room UUID to spectate the match in real-time.
					</p>

					<div>
						<input
							type="text"
							bind:value={spectateCode}
							placeholder="Enter Room Code or UUID"
							class="w-full rounded-[var(--radius-sm)] border border-[var(--bg-border)] bg-[var(--bg-elevated)] p-3 font-mono text-sm text-[var(--text-primary)] uppercase focus:border-[var(--accent)] focus:outline-none"
						/>
					</div>

					<div class="text-[11px] text-[var(--text-muted)]">
						Spectators see live tactic submission events, verdicts, and final proofs without
						disrupting players.
					</div>

					<Button
						variant="primary"
						size="md"
						disabled={!spectateCode.trim()}
						onclick={handleSpectate}
						class="w-full"
					>
						Watch Match Live
					</Button>
				</div>
			{/if}

			<Button variant="secondary" size="md" onclick={handleCancel} class="w-full">
				Return to Main Menu
			</Button>
		{/if}
	{/if}
</div>
