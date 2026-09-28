<script lang="ts">
	import '../app.css';
	import favicon from '$lib/assets/favicon.svg';
	import { MockWebSocket } from '$lib/mock/server';

	if (typeof window !== 'undefined') {
		const params = new URLSearchParams(window.location.search);
		if (params.has('mock') || sessionStorage.getItem('proofbattle.mock') === '1') {
			sessionStorage.setItem('proofbattle.mock', '1');
			MockWebSocket.install();
		}
	}

	let { children } = $props();
</script>

<svelte:head>
	<title>ProofBattle — Competitive Lean Theorem Proving</title>
	<link rel="icon" href={favicon} />
</svelte:head>

<a
	href="#main-content"
	class="sr-only focus:not-sr-only focus:fixed focus:top-3 focus:left-3 focus:z-50 focus:rounded-[var(--radius-md)] focus:bg-[var(--accent)] focus:px-4 focus:py-2 focus:text-white focus:shadow-lg focus:outline-none"
>
	Skip to content
</a>

<div
	class="flex min-h-screen flex-col bg-[var(--bg-base)] font-[var(--font-ui)] text-[var(--text-primary)]"
>
	<main id="main-content" class="flex flex-1 flex-col">
		{@render children?.()}
	</main>
</div>
