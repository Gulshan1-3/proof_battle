<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		open?: boolean;
		title?: string;
		onclose?: () => void;
		children?: Snippet;
		footer?: Snippet;
		class?: string;
	}

	let {
		open = $bindable(false),
		title = '',
		onclose,
		children,
		footer,
		class: className = ''
	}: Props = $props();

	let modalElement: HTMLDivElement | null = $state(null);

	function handleKeyDown(e: KeyboardEvent) {
		if (!open) return;

		if (e.key === 'Escape') {
			e.preventDefault();
			closeModal();
			return;
		}

		if (e.key === 'Tab' && modalElement) {
			const focusables = modalElement.querySelectorAll<HTMLElement>(
				'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
			);
			if (focusables.length === 0) return;

			const first = focusables[0];
			const last = focusables[focusables.length - 1];

			if (e.shiftKey && document.activeElement === first && last) {
				e.preventDefault();
				last.focus();
			} else if (!e.shiftKey && document.activeElement === last && first) {
				e.preventDefault();
				first.focus();
			}
		}
	}

	function closeModal() {
		open = false;
		onclose?.();
	}

	$effect(() => {
		if (open) {
			const originalOverflow = document.body.style.overflow;
			document.body.style.overflow = 'hidden';
			// Focus modal container or first focusable
			setTimeout(() => {
				if (modalElement) {
					const first = modalElement.querySelector<HTMLElement>(
						'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
					);
					first?.focus();
				}
			}, 50);

			return () => {
				document.body.style.overflow = originalOverflow;
			};
		}
	});
</script>

<svelte:window onkeydown={handleKeyDown} />

{#if open}
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-[rgba(0,0,0,0.75)] p-4 backdrop-blur-xs"
		role="presentation"
		onclick={(e) => {
			if (e.target === e.currentTarget) closeModal();
		}}
	>
		<div
			bind:this={modalElement}
			class="panel w-full max-w-lg overflow-hidden rounded-[var(--radius-lg)] border border-[var(--bg-border)] bg-[var(--bg-surface)] shadow-2xl {className}"
			role="dialog"
			aria-modal="true"
			aria-label={title || 'Modal dialog'}
		>
			{#if title}
				<div class="flex items-center justify-between border-b border-[var(--bg-border)] px-6 py-4">
					<h2 class="text-lg font-bold text-[var(--text-primary)]">{title}</h2>
					<button
						type="button"
						onclick={closeModal}
						class="cursor-pointer rounded-[var(--radius-sm)] p-1 text-[var(--text-secondary)] hover:text-[var(--text-primary)] focus-visible:ring-2 focus-visible:ring-[var(--accent)] focus-visible:outline-none"
						aria-label="Close dialog"
					>
						<svg
							width="20"
							height="20"
							viewBox="0 0 24 24"
							fill="none"
							stroke="currentColor"
							stroke-width="2"
						>
							<line x1="18" y1="6" x2="6" y2="18"></line>
							<line x1="6" y1="6" x2="18" y2="18"></line>
						</svg>
					</button>
				</div>
			{/if}

			<div class="px-6 py-5 text-[var(--text-primary)]">
				{@render children?.()}
			</div>

			{#if footer}
				<div
					class="flex items-center justify-end gap-3 border-t border-[var(--bg-border)] bg-[var(--bg-elevated)] px-6 py-4"
				>
					{@render footer()}
				</div>
			{/if}
		</div>
	</div>
{/if}
