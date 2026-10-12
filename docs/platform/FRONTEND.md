# ProofBattle Frontend Architecture

The frontend client is built with SvelteKit 2 and Svelte 5 runes (`frontend/`), delivering a responsive, low-latency, and zero-CDN competitive mathematics experience.

---

## 1. Svelte 5 Runes Reactive Architecture (ADR-005)

ProofBattle migrated from legacy Svelte 4 reactivity (`$:`, `export let`, `createEventDispatcher`) to modern **Svelte 5 Runes**:

- `$state(...)`: Declares deeply reactive client state (e.g., active room state, editor content, connection status).
- `$derived(...)`: Computes derived values reactively (e.g., formatted countdown timers, error squiggles, button enable states).
- `$effect(...)`: Manages external synchronization (e.g., WebSocket message dispatching, Monaco editor markers update).
- `$props()`: Type-safe, strict component properties with explicit callback props (`onclick`, `onsubmit`).

---

## 2. Zero-CDN Monaco Editor Bundling (ADR-011)

Loading Monaco Editor via `@monaco-editor/loader` introduces runtime CDN dependencies, violates strict Content Security Policies (CSP), and breaks offline development.

ProofBattle bundles Monaco Editor locally:
- Directly installed via npm (`monaco-editor`).
- Vite worker imports (`?worker`) bundle the web worker bundle directly into the production build.
- Full offline support with custom syntax tokens for Lean 4 keywords and mathematical operators.

---

## 3. Editor UX & Diagnostic Feedback Panel

```text
+--------------------------------------------------------------------------+
|  Theorem Header: theorem add_comm (a b : ℕ) : a + b = b + a := by         |
+--------------------------------------------------------------------------+
|  Unicode Palette: [ ∀ ] [ ∃ ] [ → ] [ ∧ ] [ ∨ ] [ ¬ ] [ ↔ ] [ ℕ ] [ ⊢ ]  |
+--------------------------------------------------------------------------+
|  Monaco Code Editor                                                      |
|  1 | intro a b                                                           |
|  2 | rw [Nat.add_comm] ~~~~~~ (Red Squiggle: Line 2, Col 4)              |
+--------------------------------------------------------------------------+
|  Compiler Diagnostic Feedback Panel                                      |
|  Line 2: type mismatch at rw [Nat.add_comm]                              |
|          found 'a + b = b + a' but expected 'Nat'                        |
+--------------------------------------------------------------------------+
```

### Automatic Diagnostic Re-Mapping
When the server returns compiler diagnostics, the error coordinates are given relative to the player's submitted tactic script (1-based lines). The frontend maps these directly onto Monaco editor marker decorations without requiring client-side log parsing.

---

## 4. Server-Authoritative Clock Interpolation (ADR-004)

To prevent client clock tampering and timer drift:
1. The server broadcasts `ends_at_ms` (Unix timestamp) upon round start.
2. The server emits `ServerTime { server_time_ms }` pulses every 15 seconds.
3. The client computes a monotonic clock offset:
   $$\text{offset} = \text{server\_time\_ms} - \text{Date.now}()$$
4. Remaining time is derived continuously:
   $$\text{remaining\_ms} = \max\left(0, \text{ends\_at\_ms} - (\text{Date.now}() + \text{offset})\right)$$

---

## 5. Key Views & Routes

| Route | View | Description |
| :--- | :--- | :--- |
| `/` | Landing & Mode Selection | Handle configuration, mode selection (Ranked, Solo Bot, Private Room). |
| `/battle/[roomId]` | 1v1 Battle Arena | Real-time split-screen duel, opponent progress indicator, Monaco editor. |
| `/history` | History Dashboard | Elo ratings, tier badges, match records, performance analytics. |
| `/replay/[matchId]` | Replay Viewer | Step-by-step replay of completed matches with canonical solutions. |

---

## 6. Code References

- Root Layout & Stores: [`frontend/src/routes/+layout.svelte`](file:///home/gulshansharma/proof_battle/frontend/src/routes/+layout.svelte)
- Battle Arena: [`frontend/src/routes/battle/[roomId]/+page.svelte`](file:///home/gulshansharma/proof_battle/frontend/src/routes/battle/[roomId]/+page.svelte)
- WebSocket Client: [`frontend/src/lib/ws/client.ts`](file:///home/gulshansharma/proof_battle/frontend/src/lib/ws/client.ts)
- Monaco Lean Editor: [`frontend/src/lib/editor/MonacoEditor.svelte`](file:///home/gulshansharma/proof_battle/frontend/src/lib/editor/MonacoEditor.svelte)
