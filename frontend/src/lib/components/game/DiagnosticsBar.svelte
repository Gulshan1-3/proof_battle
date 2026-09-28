<script lang="ts">
	import type { Diagnostic } from '$lib/ws/generated';

	interface Props {
		diagnostics?: Diagnostic[];
		checking?: boolean;
		onjump?: (line: number, col: number) => void;
	}

	let { diagnostics = [], checking = false, onjump }: Props = $props();

	let expandedIndex = $state<number | null>(null);

	const visibleDiagnostics = $derived(diagnostics.slice(0, 5));
	const hiddenCount = $derived(Math.max(0, diagnostics.length - 5));

	function toggleExpand(idx: number, e: MouseEvent) {
		e.stopPropagation();
		expandedIndex = expandedIndex === idx ? null : idx;
	}

	function handleJump(d: Diagnostic) {
		onjump?.(d.line, d.col);
	}
</script>

<div
	class="border-t border-[var(--bg-border)] bg-[var(--bg-surface)] text-xs text-[var(--text-secondary)] select-none"
	role="region"
	aria-label="Compiler Diagnostics"
>
	<!-- Status header if checking or empty -->
	{#if checking}
		<div
			class="flex items-center gap-2 px-3 py-1.5 font-mono text-[11px] text-[var(--accent-bright)]"
		>
			<span class="inline-block h-2 w-2 animate-ping rounded-full bg-[var(--accent)]"></span>
			<span>Checking proof tactics…</span>
		</div>
	{:else if diagnostics.length === 0}
		<div
			class="flex items-center justify-between px-3 py-1 font-mono text-[11px] text-[var(--text-muted)]"
		>
			<span class="flex items-center gap-1.5 text-emerald-400">
				<svg
					width="12"
					height="12"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2.5"
				>
					<polyline points="20 6 9 17 4 12"></polyline>
				</svg>
				No compiler errors detected
			</span>
			<span class="text-[10px]">Ready</span>
		</div>
	{:else}
		<div class="flex flex-col divide-y divide-[var(--bg-border)]">
			<!-- Header summary bar -->
			<div
				class="flex items-center justify-between bg-[var(--bg-elevated)] px-3 py-1 font-mono text-[11px]"
			>
				<span class="font-bold text-[var(--text-primary)]">
					{diagnostics.length} Diagnostic{diagnostics.length === 1 ? '' : 's'}
				</span>
				{#if hiddenCount > 0}
					<span
						class="rounded bg-[var(--bg-overlay)] px-1.5 py-0.5 text-[10px] text-[var(--text-muted)]"
					>
						+{hiddenCount} more
					</span>
				{/if}
			</div>

			<!-- Visible diagnostics list -->
			{#each visibleDiagnostics as d, idx (idx)}
				{@const firstLine = d.message.split('\n')[0]}
				{@const hasMultipleLines = d.message.includes('\n')}
				{@const isExpanded = expandedIndex === idx}

				<div
					class="group flex flex-col transition-colors hover:bg-[var(--bg-elevated)]"
					role="button"
					tabindex="0"
					onclick={() => handleJump(d)}
					onkeydown={(e) => e.key === 'Enter' && handleJump(d)}
				>
					<div class="flex items-center justify-between gap-2 px-3 py-1.5">
						<div class="flex min-w-0 items-center gap-2">
							<!-- Severity Badge -->
							{#if d.severity === 'error'}
								<span
									class="flex h-4 w-4 shrink-0 items-center justify-center rounded-full bg-red-500/20 text-red-400"
									title="Error"
								>
									<svg
										width="10"
										height="10"
										viewBox="0 0 24 24"
										fill="none"
										stroke="currentColor"
										stroke-width="3"
									>
										<line x1="18" y1="6" x2="6" y2="18"></line>
										<line x1="6" y1="6" x2="18" y2="18"></line>
									</svg>
								</span>
							{:else if d.severity === 'warning'}
								<span
									class="flex h-4 w-4 shrink-0 items-center justify-center rounded-full bg-amber-500/20 text-amber-400"
									title="Warning"
								>
									<svg
										width="10"
										height="10"
										viewBox="0 0 24 24"
										fill="none"
										stroke="currentColor"
										stroke-width="3"
									>
										<polygon points="12 2 2 22 22 22"></polygon>
										<line x1="12" y1="9" x2="12" y2="13"></line>
										<line x1="12" y1="17" x2="12.01" y2="17"></line>
									</svg>
								</span>
							{:else}
								<span
									class="flex h-4 w-4 shrink-0 items-center justify-center rounded-full bg-blue-500/20 text-blue-400"
									title="Info"
								>
									<svg
										width="10"
										height="10"
										viewBox="0 0 24 24"
										fill="none"
										stroke="currentColor"
										stroke-width="3"
									>
										<circle cx="12" cy="12" r="10"></circle>
										<line x1="12" y1="16" x2="12" y2="12"></line>
										<line x1="12" y1="8" x2="12.01" y2="8"></line>
									</svg>
								</span>
							{/if}

							<!-- Line:Col Jump Link -->
							<span
								class="shrink-0 rounded bg-[var(--bg-overlay)] px-1.5 py-0.5 font-mono text-[10px] font-semibold text-[var(--accent-bright)] group-hover:underline"
								title="Click to jump to line {d.line}, column {d.col}"
							>
								{d.line}:{d.col}
							</span>

							<!-- Message snippet (first line) -->
							<span class="truncate font-mono text-xs text-[var(--text-primary)]">
								{firstLine}
							</span>
						</div>

						<!-- Expand button if multi-line -->
						{#if hasMultipleLines}
							<button
								type="button"
								class="shrink-0 p-1 text-[var(--text-muted)] hover:text-[var(--text-primary)]"
								title={isExpanded ? 'Collapse' : 'Expand full message'}
								onclick={(e) => toggleExpand(idx, e)}
							>
								<svg
									width="12"
									height="12"
									viewBox="0 0 24 24"
									fill="none"
									stroke="currentColor"
									stroke-width="2"
									class="transition-transform {isExpanded ? 'rotate-180' : ''}"
								>
									<polyline points="6 9 12 15 18 9"></polyline>
								</svg>
							</button>
						{/if}
					</div>

					<!-- Expanded full diagnostic view -->
					{#if isExpanded}
						<pre
							class="border-t border-[var(--bg-border)] bg-[var(--editor-bg)] px-3 py-2 font-mono text-[11px] whitespace-pre-wrap text-[var(--text-secondary)]">{d.message}</pre>
					{/if}
				</div>
			{/each}
		</div>
	{/if}
</div>
