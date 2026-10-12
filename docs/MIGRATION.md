# Svelte 4 to Svelte 5 Migration Guide (ProofBattle)

This document establishes the exact normative mapping from `frontend_design.md` (which used legacy Svelte 4 idioms) to **Svelte 5 Runes** as mandated by **ADR-005**. Every developer and AI assistant working on `proof_battle/frontend` must follow these rules.

---

## 1. Syntax Mapping Table

| Svelte 4 Legacy Idiom                                                 | Svelte 5 Runes Idiom                                                                               | Notes & Invariants                                                                   |
| :-------------------------------------------------------------------- | :------------------------------------------------------------------------------------------------- | :----------------------------------------------------------------------------------- |
| `export let x: string = ''`                                           | `let { x = '' }: Props = $props()`                                                                 | Props are received via `$props()`. Use `$bindable()` if two-way binding is required. |
| `export let x = $bindable()`                                          | `let { x = $bindable('') }: Props = $props()`                                                      | Allows parent `bind:x={val}`.                                                        |
| `$: y = a + b`                                                        | `const y = $derived(a + b)`                                                                        | Read-only reactive calculation. Cannot be reassigned.                                |
| `$: { console.log(x); doEffect(); }`                                  | `$effect(() => { console.log(x); doEffect(); })`                                                   | Runs on mount and whenever read dependencies change. Must not mutate state directly. |
| `on:click={handleClick}`                                              | `onclick={handleClick}`                                                                            | Native HTML event attribute naming (`onclick`, `onkeydown`, `onsubmit`, etc.).       |
| `on:change={handleChange}`                                            | `oninput={handleInput}` / `onchange={handleChange}`                                                | Standard DOM event naming.                                                           |
| `const dispatch = createEventDispatcher(); dispatch('submit', data);` | Callback props: `let { onsubmit }: { onsubmit?: (data: T) => void } = $props(); onsubmit?.(data);` | Replaces event dispatchers with type-safe callback functions.                        |
| `<slot />`                                                            | `let { children }: { children?: Snippet } = $props(); {@render children?.()}`                      | Replaces default slots with Svelte 5 snippets.                                       |
| `<slot name="header" />`                                              | `let { header }: { header?: Snippet } = $props(); {@render header?.()}`                            | Named slots are now snippet props.                                                   |
| `<svelte:fragment slot="header">`                                     | `{#snippet header()}...{/snippet}`                                                                 | Callers pass snippets directly into named snippet blocks.                            |

---

## 2. Before & After Code Examples

### 2.1 Props & Two-Way Binding

#### Before (Svelte 4):

```svelte
<script lang="ts">
	export let value: string = '';
	export let readonly: boolean = false;
</script>

<input {readonly} bind:value />
```

#### After (Svelte 5):

```svelte
<script lang="ts">
	interface Props {
		value?: string;
		readonly?: boolean;
	}

	let { value = $bindable(''), readonly = false }: Props = $props();
</script>

<input {readonly} bind:value />
```

---

### 2.2 Derived Values vs State

#### Rule:

- `$state(init)`: Represents reactive, reassignable state.
- `$derived(expr)`: Represents a pure computed projection of reactive dependencies. **Never reassign a `$derived` variable**.

#### Before (Svelte 4):

```svelte
<script lang="ts">
	export let durationMs: number = 300_000;
	export let elapsedMs: number = 0;

	$: remainingMs = Math.max(0, durationMs - elapsedMs);
	$: isUrgent = remainingMs <= 60_000;
</script>
```

#### After (Svelte 5):

```svelte
<script lang="ts">
	interface Props {
		durationMs?: number;
		elapsedMs?: number;
	}

	let { durationMs = 300_000, elapsedMs = 0 }: Props = $props();

	const remainingMs = $derived(Math.max(0, durationMs - elapsedMs));
	const isUrgent = $derived(remainingMs <= 60_000);
</script>
```

---

### 2.3 Side Effects (`$effect`)

#### Before (Svelte 4):

```svelte
<script lang="ts">
	export let diagnostics: Diagnostic[] = [];

	$: if (editor && diagnostics) {
		applyDiagnostics(diagnostics);
	}
</script>
```

#### After (Svelte 5):

```svelte
<script lang="ts">
	interface Props {
		diagnostics?: Diagnostic[];
	}

	let { diagnostics = [] }: Props = $props();

	$effect(() => {
		if (editor && diagnostics) {
			applyDiagnostics(diagnostics);
		}
	});
</script>
```

---

### 2.4 Event Handling & Callback Props

#### Before (Svelte 4):

```svelte
<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	const dispatch = createEventDispatcher<{ submit: string }>();

	function handleClick() {
		dispatch('submit', 'code');
	}
</script>

<button on:click={handleClick}>Submit</button>
```

#### After (Svelte 5):

```svelte
<script lang="ts">
	interface Props {
		onsubmit?: (code: string) => void;
	}

	let { onsubmit }: Props = $props();

	function handleClick() {
		onsubmit?.('code');
	}
</script>

<button onclick={handleClick}>Submit</button>
```

---

### 2.5 Slots & Snippets

#### Before (Svelte 4):

```svelte
<!-- Modal.svelte -->
<div class="modal">
	<div class="modal-header">
		<slot name="title" />
	</div>
	<div class="modal-body">
		<slot />
	</div>
</div>

<!-- Parent.svelte -->
<Modal>
	<svelte:fragment slot="title">Victory!</svelte:fragment>
	<p>You solved the theorem.</p>
</Modal>
```

#### After (Svelte 5):

```svelte
<!-- Modal.svelte -->
<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		title?: Snippet;
		children?: Snippet;
	}

	let { title, children }: Props = $props();
</script>

<div class="modal">
	<div class="modal-header">
		{@render title?.()}
	</div>
	<div class="modal-body">
		{@render children?.()}
	</div>
</div>

<!-- Parent.svelte -->
<Modal>
	{#snippet title()}Victory!{/snippet}
	<p>You solved the theorem.</p>
</Modal>
```

---

## 3. Store Pattern $\rightarrow$ Svelte 5 Rune Classes

In Svelte 4, global stores relied on `writable()` from `svelte/store` with `$store` auto-subscription in `.svelte` components.

In Svelte 5, the recommended pattern for app state is a **Rune Class** declared in a `.svelte.ts` file. This eliminates boilerplate subscriptions and works everywhere:

```typescript
// src/lib/stores/game.svelte.ts
export class GameState {
	playerId = $state<string | null>(null);
	roundActive = $state(false);
	endsAtMs = $state(0);
	serverTimeOffsetMs = $state(0);

	timeRemainingMs = $derived(Math.max(0, this.endsAtMs - (Date.now() + this.serverTimeOffsetMs)));

	canSubmit = $derived(this.roundActive && this.timeRemainingMs > 0);

	reset() {
		this.playerId = null;
		this.roundActive = false;
		this.endsAtMs = 0;
	}
}

export const game = new GameState();
```
