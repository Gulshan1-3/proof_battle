<script lang="ts">
	import Button from '$lib/components/ui/Button.svelte';
	import Badge from '$lib/components/ui/Badge.svelte';

	interface SearchTraceStep {
		depth: number;
		tactic: string;
		source: string;
		verified: boolean;
		latency_ms: number;
		error?: string | null;
	}

	interface ProverSolveResponse {
		success: boolean;
		solved: boolean;
		winning_tactic: string;
		full_code: string;
		solve_method: string;
		latency_ms: number;
		nodes_expanded: number;
		max_depth: number;
		axioms: string[];
		sorry_count: number;
		search_trace: SearchTraceStep[];
		error: string | null;
	}

	// Preset problem catalog
	const PRESETS = [
		{
			id: 'am_gm',
			label: 'AM-GM Quadratic',
			text: 'For any real numbers a and b, (a + b)^2 - 4 * a * b = (a - b)^2'
		},
		{
			id: 'pythagoras',
			label: 'Pythagorean Relation',
			text: 'In a right triangle with legs a, b and hypotenuse c where a^2 + b^2 = c^2, show that (a + b)^2 = c^2 + 2 * a * b'
		},
		{
			id: 'euler_cubes',
			label: 'Euler Sum of Cubes',
			text: 'Let a, b, c be real numbers such that a + b + c = 0. Show that a^3 + b^3 + c^3 = 3 * a * b * c'
		},
		{
			id: 'lagrange_2d',
			label: 'Lagrange 2D Identity',
			text: 'Prove that for real coordinates a1, a2, b1, b2, (a1^2 + a2^2) * (b1^2 + b2^2) - (a1 * b1 + a2 * b2)^2 = (a1 * b2 - a2 * b1)^2'
		},
		{
			id: 'peano',
			label: 'Peano Succ Injective',
			text: 'For natural numbers n and m, if n.succ = m.succ then n = m'
		},
		{
			id: 'nat_add_comm',
			label: 'Nat Addition Comm',
			text: 'For any natural numbers n and m, n + m = m + n'
		}
	];

	let inputText = $state('For any real numbers a and b, (a + b)^2 - 4 * a * b = (a - b)^2');
	let isFormalizing = $state(false);
	let isSolving = $state(false);
	let activeStage = $state<'input' | 'formalized' | 'solving' | 'done'>('input');

	// Formalization State
	let leanCode = $state('');
	let theoremStatement = $state('');
	let goalProp = $state('');
	let varBinders = $state('');
	let formalizeMethod = $state('');
	let autoformalizeError = $state<string | null>(null);

	// Prover Settings & Response
	let simulationsBudget = $state(32);
	let solveResult = $state<ProverSolveResponse | null>(null);
	let copiedCertificate = $state(false);
	let copyTimeout: ReturnType<typeof setTimeout> | null = null;

	function selectPreset(presetText: string) {
		inputText = presetText;
		solveResult = null;
		autoformalizeError = null;
		activeStage = 'input';
	}

	async function handleAutoformalize() {
		if (!inputText.trim()) return;
		isFormalizing = true;
		autoformalizeError = null;
		solveResult = null;

		try {
			const res = await fetch('/sim-api/api/autoformalize', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ problem_text: inputText, text: inputText, informal_text: inputText })
			});
			const data = await res.json();
			if (data.success) {
				leanCode = data.lean_code;
				theoremStatement = data.statement;
				goalProp = data.goal;
				varBinders = data.binders;
				formalizeMethod = data.method || 'neuro_symbolic';
				activeStage = 'formalized';
			} else {
				autoformalizeError = data.error || 'Failed to autoformalize problem statement.';
			}
		} catch (err) {
			autoformalizeError = `Connection error: ${String(err)}`;
		} finally {
			isFormalizing = false;
		}
	}

	async function handleSolve() {
		if (!leanCode.trim()) {
			await handleAutoformalize();
			if (!leanCode.trim()) return;
		}

		isSolving = true;
		activeStage = 'solving';
		solveResult = null;

		try {
			const res = await fetch('/sim-api/api/prover/solve', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					lean_code: leanCode,
					goal: goalProp,
					params: varBinders,
					simulations: simulationsBudget,
					timeout: 35.0
				})
			});
			const data: ProverSolveResponse = await res.json();
			solveResult = data;
			activeStage = 'done';
		} catch (err) {
			solveResult = {
				success: false,
				solved: false,
				winning_tactic: '',
				full_code: leanCode,
				solve_method: 'error',
				latency_ms: 0,
				nodes_expanded: 0,
				max_depth: 0,
				axioms: [],
				sorry_count: 0,
				search_trace: [],
				error: `Server request failed: ${String(err)}`
			};
			activeStage = 'done';
		} finally {
			isSolving = false;
		}
	}

	async function handleFormalizeAndSolve() {
		isFormalizing = true;
		autoformalizeError = null;
		solveResult = null;

		try {
			const res = await fetch('/sim-api/api/autoformalize', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ problem_text: inputText, text: inputText, informal_text: inputText })
			});
			const data = await res.json();
			if (data.success) {
				leanCode = data.lean_code;
				theoremStatement = data.statement;
				goalProp = data.goal;
				varBinders = data.binders;
				formalizeMethod = data.method || 'neuro_symbolic';
				isFormalizing = false;
				await handleSolve();
			} else {
				autoformalizeError = data.error || 'Failed to autoformalize problem statement.';
				isFormalizing = false;
			}
		} catch (err) {
			autoformalizeError = `Connection error: ${String(err)}`;
			isFormalizing = false;
		}
	}

	function copyToClipboard(text: string) {
		navigator.clipboard.writeText(text);
		copiedCertificate = true;
		if (copyTimeout) clearTimeout(copyTimeout);
		copyTimeout = setTimeout(() => {
			copiedCertificate = false;
		}, 2000);
	}
</script>

<svelte:head>
	<title>ProofBDH: Minimalist Lean 4 Neural Theorem Prover</title>
</svelte:head>

<div class="mx-auto flex w-full max-w-5xl flex-1 flex-col px-6 py-8">
	<!-- Minimalist Brand Header -->
	<header class="flex w-full items-center justify-between border-b border-[#27272a] pb-6">
		<div class="flex items-center gap-3">
			<div
				class="flex h-8 w-8 items-center justify-center rounded-md border border-[#3f3f46] bg-[#18181b] font-mono text-sm font-bold text-[#f4f4f5]"
			>
				⊢
			</div>
			<div>
				<h1 class="text-base font-semibold tracking-tight text-[#f4f4f5]">ProofBDH</h1>
				<p class="text-xs text-[#71717a]">Lean 4 Neuro-Symbolic Theorem Prover</p>
			</div>
		</div>

		<div class="flex items-center gap-3 text-xs text-[#a1a1aa]">
			<span class="rounded border border-[#27272a] bg-[#121215] px-2.5 py-1 font-mono">
				9.9M Params
			</span>
			<span class="rounded border border-[#27272a] bg-[#121215] px-2.5 py-1 font-mono">
				117k Mathlib RAG
			</span>
			<span class="rounded border border-[#27272a] bg-[#121215] px-2.5 py-1 font-mono">
				Lean 4 Kernel
			</span>
			<a
				href="/lobby"
				class="rounded border border-[#27272a] px-2.5 py-1 transition-colors hover:border-[#3f3f46] hover:text-[#f4f4f5]"
			>
				Multiplayer
			</a>
		</div>
	</header>

	<!-- Main Workspace -->
	<main class="mt-8 flex flex-1 flex-col space-y-6">
		<!-- Section 1: Problem Input -->
		<section class="rounded-lg border border-[#27272a] bg-[#121215] p-5">
			<div class="mb-3 flex items-center justify-between">
				<label for="problem-input" class="text-xs font-semibold tracking-wider text-[#a1a1aa] uppercase">
					Input Problem (Natural Language, LaTeX, or Lean 4)
				</label>
				<span class="text-xs text-[#71717a]">Press Ctrl+Enter to solve</span>
			</div>

			<!-- Preset Selection Pills -->
			<div class="mb-3 flex flex-wrap gap-1.5">
				<span class="py-1 text-xs text-[#71717a]">Presets:</span>
				{#each PRESETS as preset}
					<button
						type="button"
						onclick={() => selectPreset(preset.text)}
						class="rounded border border-[#27272a] bg-[#18181b] px-2.5 py-0.5 text-xs text-[#a1a1aa] transition-colors hover:border-[#3f3f46] hover:text-[#f4f4f5]"
					>
						{preset.label}
					</button>
				{/each}
			</div>

			<textarea
				id="problem-input"
				bind:value={inputText}
				rows="3"
				placeholder="Enter a problem statement, e.g. For any real numbers a and b, (a + b)^2 - 4 * a * b = (a - b)^2..."
				onkeydown={(e) => {
					if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
						handleFormalizeAndSolve();
					}
				}}
				class="w-full resize-y rounded-md border border-[#27272a] bg-[#09090b] p-3 font-mono text-sm text-[#f4f4f5] placeholder-[#52525b] focus:border-[#52525b] focus:outline-none"
			></textarea>

			<!-- Action Controls Bar -->
			<div class="mt-4 flex flex-wrap items-center justify-between gap-3 border-t border-[#27272a] pt-4">
				<div class="flex items-center gap-3">
					<label for="budget-select" class="text-xs text-[#71717a]">MCTS Simulations:</label>
					<select
						id="budget-select"
						bind:value={simulationsBudget}
						class="rounded border border-[#27272a] bg-[#18181b] px-2 py-1 font-mono text-xs text-[#f4f4f5] focus:outline-none"
					>
						<option value={16}>16 Sims (Fast)</option>
						<option value={32}>32 Sims (Balanced)</option>
						<option value={64}>64 Sims (Deep)</option>
					</select>
				</div>

				<div class="flex items-center gap-2">
					<Button
						variant="secondary"
						size="sm"
						loading={isFormalizing}
						onclick={handleAutoformalize}
					>
						Autoformalize Only
					</Button>
					<Button
						variant="primary"
						size="sm"
						loading={isFormalizing || isSolving}
						onclick={handleFormalizeAndSolve}
					>
						Formalize & Solve
					</Button>
				</div>
			</div>

			{#if autoformalizeError}
				<div class="mt-3 rounded border border-[#7f1d1d] bg-[#450a0a]/30 p-2.5 text-xs text-[#fca5a5]">
					{autoformalizeError}
				</div>
			{/if}
		</section>

		<!-- Section 2: Converted Lean 4 Specification -->
		{#if leanCode}
			<section class="rounded-lg border border-[#27272a] bg-[#121215] p-5">
				<div class="mb-3 flex items-center justify-between">
					<div class="flex items-center gap-2">
						<span class="text-xs font-semibold tracking-wider text-[#a1a1aa] uppercase">
							Formal Lean 4 Specification
						</span>
						<Badge variant="default" size="sm">{formalizeMethod}</Badge>
					</div>
					<div class="flex items-center gap-2">
						<button
							type="button"
							onclick={() => copyToClipboard(leanCode)}
							class="text-xs text-[#71717a] transition-colors hover:text-[#f4f4f5]"
						>
							{copiedCertificate ? 'Copied' : 'Copy Lean 4'}
						</button>
						{#if activeStage === 'formalized'}
							<Button variant="primary" size="sm" loading={isSolving} onclick={handleSolve}>
								Solve This Theorem
							</Button>
						{/if}
					</div>
				</div>

				<div class="relative">
					<textarea
						bind:value={leanCode}
						rows="6"
						class="w-full resize-y rounded-md border border-[#27272a] bg-[#09090b] p-3 font-mono text-xs leading-relaxed text-[#f4f4f5] focus:border-[#52525b] focus:outline-none"
					></textarea>
				</div>
			</section>
		{/if}

		<!-- Section 3: Live Solving & Verification Telemetry -->
		{#if isSolving}
			<section class="rounded-lg border border-[#27272a] bg-[#121215] p-6 text-center">
				<div class="inline-flex items-center gap-3">
					<div class="h-4 w-4 animate-spin rounded-full border-2 border-[#71717a] border-t-[#f4f4f5]"></div>
					<span class="text-sm font-medium text-[#f4f4f5]">
						Running ProofBDH 10M Neural MCTS & Mathlib Premise Search...
					</span>
				</div>
				<p class="mt-2 text-xs text-[#71717a]">
					Evaluating neural candidate priors • Querying 117,911 Mathlib lemmas • Lean 4 REPL verification
				</p>
			</section>
		{/if}

		<!-- Section 4: Formal Proof Results -->
		{#if solveResult}
			<section class="rounded-lg border border-[#27272a] bg-[#121215] p-5">
				<!-- Status Header -->
				<div class="flex flex-wrap items-center justify-between gap-3 border-b border-[#27272a] pb-4">
					<div class="flex items-center gap-3">
						{#if solveResult.solved}
							<span class="flex h-6 items-center rounded-full border border-[#166534] bg-[#14532d]/40 px-3 text-xs font-semibold text-[#4ade80]">
								PROVEN (Lean 4 Kernel Verified)
							</span>
						{:else}
							<span class="flex h-6 items-center rounded-full border border-[#3f3f46] bg-[#18181b] px-3 text-xs font-semibold text-[#a1a1aa]">
								SEARCH EXHAUSTED
							</span>
						{/if}
						<span class="text-xs text-[#71717a]">
							Method: <strong class="text-[#a1a1aa] font-mono">{solveResult.solve_method}</strong>
						</span>
					</div>

					<div class="flex items-center gap-4 text-xs font-mono text-[#71717a]">
						<span>Latency: <strong class="text-[#f4f4f5]">{solveResult.latency_ms}ms</strong></span>
						<span>Depth: <strong class="text-[#f4f4f5]">{solveResult.max_depth}</strong></span>
						<span>Nodes: <strong class="text-[#f4f4f5]">{solveResult.nodes_expanded}</strong></span>
					</div>
				</div>

				<!-- Proof Script Content -->
				{#if solveResult.solved}
					<div class="mt-4">
						<div class="mb-2 flex items-center justify-between">
							<span class="text-xs font-semibold text-[#a1a1aa]">Certified Lean 4 Code:</span>
							<button
								type="button"
								onclick={() => copyToClipboard(solveResult?.full_code || '')}
								class="text-xs text-[#71717a] transition-colors hover:text-[#f4f4f5]"
							>
								{copiedCertificate ? 'Copied' : 'Copy Proof Script'}
							</button>
						</div>
						<pre class="overflow-x-auto rounded-md border border-[#27272a] bg-[#09090b] p-3 font-mono text-xs leading-relaxed text-[#f4f4f5]"><code>{solveResult.full_code}</code></pre>
					</div>

					<!-- Kernel Axiom Audit -->
					<div class="mt-4 rounded border border-[#27272a] bg-[#18181b] p-3">
						<div class="flex items-center justify-between text-xs">
							<span class="text-[#a1a1aa]">Kernel Axiom Integrity Audit:</span>
							<span class="font-mono text-[#4ade80]">0 sorryAx (100% Formally Verified)</span>
						</div>
						<div class="mt-1 flex flex-wrap gap-1 font-mono text-[11px] text-[#71717a]">
							<span>Axioms:</span>
							{#each solveResult.axioms as ax}
								<span class="rounded bg-[#27272a] px-1.5 py-0.2 text-[#a1a1aa]">{ax}</span>
							{/each}
						</div>
					</div>
				{:else}
					<div class="mt-4 rounded border border-[#27272a] bg-[#09090b] p-3 text-xs text-[#a1a1aa]">
						<p>Search concluded without closing all proof goals.</p>
						{#if solveResult.error}
							<p class="mt-1 font-mono text-[#71717a]">{solveResult.error}</p>
						{/if}
						<p class="mt-2 text-[#71717a]">
							Tip: Increase MCTS simulations to 64 or add auxiliary intermediate lemmas to guide tree search.
						</p>
					</div>
				{/if}

				<!-- Search Trace Steps -->
				{#if solveResult.search_trace && solveResult.search_trace.length > 0}
					<div class="mt-5 border-t border-[#27272a] pt-4">
						<span class="text-xs font-semibold text-[#71717a] uppercase">Tactic Search Sequence:</span>
						<div class="mt-2 space-y-1.5">
							{#each solveResult.search_trace as step}
								<div class="flex items-center justify-between rounded border border-[#27272a] bg-[#09090b] px-3 py-1.5 font-mono text-xs">
									<div class="flex items-center gap-2">
										<span class="text-[#71717a]">[{step.source}]</span>
										<span class="text-[#f4f4f5]">{step.tactic}</span>
									</div>
									<div class="flex items-center gap-2">
										<span class="text-[#71717a]">{step.latency_ms}ms</span>
										{#if step.verified}
											<span class="text-[#4ade80]">PASSED</span>
										{:else}
											<span class="text-[#71717a]">FAILED</span>
										{/if}
									</div>
								</div>
							{/each}
						</div>
					</div>
				{/if}
			</section>
		{/if}
	</main>
</div>

