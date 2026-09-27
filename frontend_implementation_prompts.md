# ProofBattle — Frontend Implementation Prompts

> **Source of truth**: `battle/frontend_design.md` (1289 lines)
> **Existing frontend**: None. The repository contains zero frontend code — no `package.json`, no SvelteKit project, no `.svelte` files, no CSS. The only HTML is a 27-line WebSocket debug harness at `battle/src/index.html`.
> **Existing backend**: Rust Axum WebSocket server at `battle/src/` (5 modules, ~390 lines total).

---

## Known Requirements

Requirements directly confirmed by `frontend_design.md`:

- **Framework**: SvelteKit (SSR + client routing)
- **Code Editor**: Monaco Editor via `@monaco-editor/loader`
- **Math Rendering**: KaTeX for theorem goal display
- **Styling**: Tailwind CSS v4 + CSS custom properties (design tokens)
- **Fonts**: JetBrains Mono (code), Inter (UI)
- **Icons**: Lucide Svelte
- **WebSocket**: Native browser WebSocket API (no library)
- **Build**: Vite (SvelteKit default)
- **Type Safety**: TypeScript throughout
- **Testing**: Playwright (e2e), Vitest (unit)
- **Three screens**: Landing, Matchmaking/Lobby, Game
- **Game layout**: Three-panel split (Problem | Editor | Status)
- **Editor theme**: Custom dark theme matching "dark math terminal meets competitive esport arena"
- **WebSocket state machine**: 9 states with explicit transitions
- **Message protocol**: JSON with `type` discriminator field (typed on both sides)

## Existing Architecture

**Rust backend** (`battle/src/`):
- `main.rs` (207 lines): Axum server on `127.0.0.1:3000`, WebSocket handler, proof submission loop
- `message.rs` (21 lines): `ClientMessage` (Join, SubmitProof) and `ServerMessage` (Joined, MatchFound, Challenge, ProofResult, GameEnded, Error)
- `room.rs` (101 lines): Matchmaker, GameRoom, Match structs with HashMap-based state
- `lean_runner.rs` (31 lines): Blocking `std::process::Command` calling `lake env lean`
- `challenges.rs` (22 lines): 3 hardcoded ProofChallenge structs
- `utils.rs` (7 lines): ASCII-to-Unicode preprocessing

**Message contract** (from `message.rs`):
- Client sends: `{ "type": "Join", "username": "..." }` or `{ "type": "SubmitProof", "code": "...", "player_id": "..." }`
- Server sends: `{ "type": "Joined", "player_id": "..." }`, `{ "type": "Challenge", "goal": "...", "imports": ["..."] }`, etc.

**Backend URL**: `ws://localhost:3000/ws`

## Assumptions

1. The frontend project will be created as a new directory `proof-battle-frontend/` at the repository root (sibling to `battle/`). The existing `battle/` directory is not a frontend project.
2. ASSUMPTION: The backend message contract in `battle/src/message.rs` is the authoritative API definition. The design doc's `messages.ts` types show an extended protocol (SessionToken, OpponentActivity, TimerUpdate, LspDiagnostic) that does NOT exist in the current backend. The frontend should implement the design doc's full protocol types but handle missing message types gracefully (they will be added to the backend later).
3. ASSUMPTION: The `player_id` field in `SubmitProof` is a known security issue (documented in `proof_battle_design.md` Section 1.5). The frontend should NOT send `player_id` — the server already knows who the sender is from the TCP connection. However, the current backend code DOES expect `player_id` in the message. The frontend will send it for now to maintain compatibility, with a comment noting this is temporary.
4. ASSUMPTION: No environment variables are needed for the frontend dev server. The WebSocket URL `ws://localhost:3000/ws` is hardcoded for local development.
5. ASSUMPTION: Tailwind CSS v4 uses the `@tailwindcss/vite` plugin (not PostCSS). This is the v4 approach.

## Open Questions

1. **ELO display on landing page**: The design shows "Live Games: 12, Players Online: 47, Top ELO: 2140" — these require a server endpoint that does not exist. Implementation will use hardcoded placeholder values.
2. **Practice mode**: The design mentions a "Practice" button but does not describe a practice mode feature. This will be a non-functional placeholder.
3. **Timer**: The design shows a countdown timer but the backend has no timer logic. The frontend will implement the timer UI but it will not be driven by server state initially.
4. **Opponent activity**: The design mentions showing "opponent is typing..." — this requires `OpponentActivity` messages from the server which do not exist yet. Will show static "Waiting..." placeholder.

---

## Step 1 — Scaffold the SvelteKit Project

### Goal

Initialize a new SvelteKit project with TypeScript, install all required dependencies, and configure Tailwind CSS v4.

### Before You Start

Inspect:

- `/home/gulshansharma/proof_battle/` — the repository root where the new project directory will be created
- `/home/gulshansharma/proof_battle/battle/` — confirm this is a Rust project, NOT a frontend project

Confirm:

- No `package.json` exists anywhere in the repository
- No `node_modules/` or frontend build artifacts exist
- The `battle/` directory contains Rust source code and Lean configuration

Do not make changes until you have confirmed no frontend project already exists.

### Context

The design document (`frontend_design.md` Section 1) specifies SvelteKit as the framework. No frontend exists in the repository. This step creates the project from scratch and installs all dependencies listed in the design doc's "Final Stack" table.

### Implementation Instructions

1. Navigate to `/home/gulshansharma/proof_battle/` (the repository root).
2. Create the SvelteKit project using `npm create svelte@latest proof-battle-frontend`. Select: Skeleton project, TypeScript, ESLint, Prettier, Playwright, Vitest.
3. `cd proof-battle-frontend` and run `npm install`.
4. Install production dependencies listed in `frontend_design.md` Section 1:
   ```bash
   npm install @monaco-editor/loader katex lucide-svelte canvas-confetti
   ```
5. Install dev dependencies:
   ```bash
   npm install -D tailwindcss @tailwindcss/vite
   ```
6. Read `proof-battle-frontend/vite.config.ts` and add the Tailwind Vite plugin:
   ```ts
   import tailwindcss from '@tailwindcss/vite';
   // Add tailwindcss() to the plugins array in defineConfig
   ```
7. Verify all dependencies are in `package.json` with correct versions.
8. Do NOT create any components, routes, or styles yet. Only scaffold and install.

### Technical Requirements

- Use SvelteKit's default project structure
- TypeScript enabled
- Vite as the build tool (SvelteKit default)
- Tailwind CSS v4 via `@tailwindcss/vite` plugin

### Edge Cases

- If `npm create svelte@latest` prompts interactively, use non-interactive flags or defaults
- If Tailwind CSS v4 API has changed from the design doc, use the latest v4 approach

### Constraints

- Do not modify any files in `battle/`
- Do not install dependencies not listed in the design doc
- Do not create components or pages yet — only scaffold

### Verification

Run:

```bash
cd proof-battle-frontend && npm run dev
```

Then verify:

- Dev server starts without errors on `http://localhost:5173`
- The default SvelteKit welcome page renders in the browser
- `npm run check` passes (TypeScript compilation)
- `npm run build` succeeds

### Completion Criteria

This step is complete when:

- `proof-battle-frontend/` directory exists with a working SvelteKit project
- All dependencies from `frontend_design.md` Section 1 are installed
- Tailwind CSS v4 is configured in `vite.config.ts`
- `npm run dev`, `npm run check`, and `npm run build` all succeed

### Expected Output

At the end of this step, the project should have:

- A new `proof-battle-frontend/` directory at the repository root
- `package.json` with all required dependencies
- A working SvelteKit dev server
- Tailwind CSS v4 configured but not yet applied

---

## Step 2 — Implement the Design Token System and Global Styles

### Goal

Create the complete CSS custom property design token system and global base styles as specified in `frontend_design.md` Section 2.

### Before You Start

Inspect:

- `proof-battle-frontend/src/app.css` — the default SvelteKit global styles
- `proof-battle-frontend/src/app.html` — the HTML shell where font links go
- `proof-battle-frontend/src/routes/+layout.svelte` — where CSS is imported

Confirm:

- The SvelteKit project was successfully scaffolded (Step 1)
- `src/app.css` exists and is imported by the layout

Do not make changes until you have confirmed the scaffold is working.

### Context

`frontend_design.md` Section 2 defines the complete design system: color palette (Section 2.1), typography (Section 2.2), spacing grid (Section 2.3), and component tokens (Section 2.4). This step establishes the foundation that ALL subsequent components will use.

### Implementation Instructions

1. Read `proof-battle-frontend/src/app.css` and `proof-battle-frontend/src/app.html`.
2. In `src/app.html`, add `<link>` tags in the `<head>` for:
   - Google Fonts: JetBrains Mono (weights 400, 600, 700)
   - Google Fonts: Inter (weights 400, 500, 600, 700)
   - KaTeX CSS: `https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/katex.min.css`
3. Replace the contents of `src/app.css` with:
   - All CSS custom properties from Section 2.1 (under `:root`):
     - Backgrounds: `--bg-base: #0d0d0f`, `--bg-surface: #141417`, `--bg-elevated: #1c1c21`, `--bg-overlay: #242429`, `--bg-border: #2a2a31`
     - Text: `--text-primary: #f0f0f5`, `--text-secondary: #8b8b9e`, `--text-muted: #4a4a5a`
     - Accent: `--accent: #7c6af7`, `--accent-dim: #7c6af720`, `--accent-bright: #9d8fff`
     - Semantic: `--success: #34d399`, `--error: #f87171`, `--warning: #fbbf24`, `--info: #60a5fa`
     - Editor: `--editor-bg: #111113`, `--editor-gutter: #1a1a1f`, `--editor-line-hl: #1e1e26`, `--editor-cursor: #7c6af7`
     - Fonts: `--font-mono: 'JetBrains Mono', 'Fira Code', monospace`, `--font-ui: 'Inter', system-ui, sans-serif`
   - All spacing tokens from Section 2.3:
     - `--space-1: 4px` through `--space-16: 64px`
   - Border radius tokens from Section 2.3:
     - `--radius-sm: 4px`, `--radius-md: 8px`, `--radius-lg: 12px`, `--radius-xl: 16px`
   - Base body styles: `font-family: var(--font-ui)`, `background: var(--bg-base)`, `color: var(--text-primary)`, `margin: 0`
   - Base code/pre styles: `font-family: var(--font-mono)`
   - Tailwind v4 import: `@import "tailwindcss";` at the very top of the file
4. Create a file `src/lib/styles/components.css` with reusable utility classes from Section 2.4:
   - `.panel`: `background: var(--bg-surface); border: 1px solid var(--bg-border); border-radius: var(--radius-lg);`
   - `.btn-primary`: full styles from Section 2.4 (background, color, padding, radius, font-weight, transition, hover/active states)
   - `.status-label`: styles from Section 2.2 (font-ui, 0.75rem, uppercase, letter-spacing 0.08em)
5. Import `components.css` in `src/app.css` after the design tokens.
6. Update `src/routes/+layout.svelte` to import `app.css` (it should already do this — verify).

### Technical Requirements

- All CSS custom properties must match the hex values from the design doc exactly
- The `@import "tailwindcss"` must be at the top of `app.css` for Tailwind v4 to work
- Fonts loaded via Google Fonts CDN (not self-hosted)

### Edge Cases

- If Tailwind v4 conflicts with custom properties, use the `@theme` directive or CSS layer system
- If KaTeX CSS version 0.16.9 is unavailable, use the latest available version

### Constraints

- Do not modify any files in `battle/`
- Do not create components yet — only tokens and base styles
- Do not use SCSS or CSS preprocessors — plain CSS only
- Preserve backward compatibility with SvelteKit's default styles

### Verification

Run:

```bash
cd proof-battle-frontend && npm run dev
```

Then verify:

- `npm run check` passes
- In browser DevTools, inspect `:root` and confirm all CSS custom properties are present
- Inspect `body` and confirm font-family is Inter, background is `#0d0d0f`, text color is `#f0f0f5`
- Confirm Google Fonts load (check Network tab for JetBrains Mono and Inter)
- Confirm KaTeX CSS loads

### Completion Criteria

This step is complete when:

- All 25+ CSS custom properties from Section 2.1 are defined
- All spacing and radius tokens from Section 2.3 are defined
- Base typography styles are applied
- `.panel`, `.btn-primary`, `.status-label` utility classes are available
- Fonts load correctly from Google Fonts CDN
- `npm run check` passes

### Expected Output

At the end of this step, the project should have:

- `src/app.css` with complete design token system
- `src/lib/styles/components.css` with reusable utility classes
- `src/app.html` with font and KaTeX CSS links
- A dark-themed page body ready for component styling

---

## Step 3 — Create the Route Structure

### Goal

Create the three SvelteKit routes (Landing, Lobby, Game) with a shared layout, as specified in `frontend_design.md` Sections 3 and 11.

### Before You Start

Inspect:

- `proof-battle-frontend/src/routes/` — the default SvelteKit routes
- `proof-battle-frontend/src/routes/+page.svelte` — the default home page
- `proof-battle-frontend/src/routes/+layout.svelte` — the default layout

Confirm:

- Design tokens are working (Step 2)
- SvelteKit routing is functional

Do not make changes until you have confirmed the design tokens are applied.

### Context

`frontend_design.md` Section 11 specifies the file structure with three routes: `+page.svelte` (landing), `lobby/+page.svelte` (matchmaking), `game/+page.svelte` (game). Section 3 provides the wireframes for each screen. This step creates the route structure with placeholder content.

### Implementation Instructions

1. Read the existing route files in `proof-battle-frontend/src/routes/`.
2. Update `src/routes/+layout.svelte`:
   - Add a header/nav bar with: `⊢ ProofBattle` text (use `var(--font-mono)`, `var(--text-primary)`)
   - Style with `var(--bg-surface)` background, `var(--bg-border)` bottom border, padding `var(--space-4) var(--space-6)`
   - Include `<slot />` for child route content
3. Update `src/routes/+page.svelte` (Landing page):
   - Heading: "ProofBattle" with subtitle "Competitive Lean Theorem Proving"
   - Placeholder content for now (actual landing page built in Step 11)
4. Create `proof-battle-frontend/src/routes/lobby/+page.svelte`:
   - Placeholder: `<h1>Finding your opponent...</h1>`
5. Create `proof-battle-frontend/src/routes/game/+page.svelte`:
   - Placeholder: `<h1>Game Screen</h1>`
6. Verify all three routes load.

### Technical Requirements

- SvelteKit file-based routing (no manual route configuration)
- Layout applies to all routes via `+layout.svelte`
- Each page is a Svelte component

### Edge Cases

- Ensure the layout does not break on routes that do not exist (SvelteKit 404 handling is automatic)

### Constraints

- Do not modify any files in `battle/`
- Do not implement actual page content — only route structure with placeholders
- Keep the layout minimal — only nav bar and slot

### Verification

Run:

```bash
cd proof-battle-frontend && npm run dev
```

Then verify:

- Navigate to `http://localhost:5173/` — shows landing page placeholder
- Navigate to `http://localhost:5173/lobby` — shows lobby placeholder
- Navigate to `http://localhost:5173/game` — shows game placeholder
- All three pages show the shared nav bar from the layout
- `npm run check` passes

### Completion Criteria

This step is complete when:

- Three routes exist: `/`, `/lobby`, `/game`
- Shared layout renders on all pages
- Each route shows its placeholder heading
- `npm run check` passes

### Expected Output

At the end of this step, the project should have:

- `src/routes/+layout.svelte` with nav bar
- `src/routes/+page.svelte` with landing placeholder
- `src/routes/lobby/+page.svelte` with lobby placeholder
- `src/routes/game/+page.svelte` with game placeholder

---

## Step 4 — Implement the WebSocket Message Types and State Machine

### Goal

Define the TypeScript types for the WebSocket message protocol and the connection state machine, as specified in `frontend_design.md` Sections 6 and 7.

### Before You Start

Inspect:

- `battle/src/message.rs` — the authoritative Rust-side message definitions
- `frontend_design.md` Section 7 — the TypeScript message types (these are the TARGET, but the Rust side is the CURRENT truth)

Confirm:

- The Rust `ClientMessage` has: `Join { username }`, `SubmitProof { code, player_id }`
- The Rust `ServerMessage` has: `Joined { player_id }`, `MatchFound { opponent }`, `Challenge { goal, imports }`, `ProofResult { success, output }`, `GameEnded { winner }`, `Error { message }`
- The design doc's TypeScript types are an EXTENDED version that the backend does not yet implement

Do not make changes until you understand the gap between the current backend API and the design doc's target API.

### Context

`frontend_design.md` Section 6 defines the WebSocket state machine (9 states, 12 events). Section 7 defines the message protocol. The current Rust backend (`message.rs`) implements a subset of these messages. The frontend should define the FULL target types from the design doc but handle unknown message types gracefully.

### Implementation Instructions

1. Read `battle/src/message.rs` to understand the current API.
2. Create `proof-battle-frontend/src/lib/ws/messages.ts`:
   - Define `ClientMessage` as a discriminated union. Include BOTH the current backend's `SubmitProof` (with `player_id`) and the target `SubmitProof` (without `player_id`). For now, use the current backend's version:
     ```ts
     export type ClientMessage =
       | { type: 'Join'; username: string }
       | { type: 'SubmitProof'; code: string; player_id: string } // TEMPORARY: backend expects this
     ```
   - Define `ServerMessage` as a discriminated union with ALL types from Section 7:
     ```ts
     export type ServerMessage =
       | { type: 'Joined'; player_id: string }
       | { type: 'MatchFound'; opponent: string }
       | { type: 'Challenge'; goal: string; imports: string[] }
       | { type: 'ProofResult'; success: boolean; output: string }
       | { type: 'GameEnded'; winner: string }
       | { type: 'SessionToken'; token: string }        // Not yet in backend
       | { type: 'OpponentActivity'; status: 'typing' | 'submitted' | 'idle' } // Not yet in backend
       | { type: 'TimerUpdate'; seconds_remaining: number } // Not yet in backend
       | { type: 'LspDiagnostic'; line: number; col: number; end_line: number; end_col: number; message: string; severity: 'error' | 'warning' | 'info' } // Not yet in backend
       | { type: 'Error'; message: string }
     ```
3. Create `proof-battle-frontend/src/lib/ws/stateMachine.ts`:
   - Define `WsState` type with all 9 states from Section 6
   - Define `WsEvent` type with all 12 event types from Section 6
   - Define `TRANSITIONS` record from Section 6
   - Implement `transition(currentState, event)` function that returns the new state
   - If a transition is invalid, throw an error with a descriptive message

### Technical Requirements

- TypeScript discriminated unions with `type` as the discriminator field
- All message types from the design doc must be defined (even if backend doesn't send them yet)
- State machine transitions must match the design doc exactly

### Edge Cases

- Unknown message types from the server should not crash the client — handle with a `default` case in the message handler
- Invalid state transitions should throw (not silently fail)

### Constraints

- Do not modify any files in `battle/`
- Do not create the WebSocket client yet — only types and state machine logic
- Use the current backend's `SubmitProof` signature (with `player_id`) for now

### Verification

Run:

```bash
cd proof-battle-frontend && npm run check
```

Then verify:

- TypeScript compilation passes
- The `transition` function correctly handles all valid transitions from the design doc
- Invalid transitions throw errors

### Completion Criteria

This step is complete when:

- `src/lib/ws/messages.ts` defines all ClientMessage and ServerMessage types
- `src/lib/ws/stateMachine.ts` defines WsState, WsEvent, TRANSITIONS, and transition function
- `npm run check` passes
- The state machine handles all 12 event types

### Expected Output

At the end of this step, the project should have:

- `src/lib/ws/messages.ts` with complete message type definitions
- `src/lib/ws/stateMachine.ts` with state machine logic

---

## Step 5 — Implement the Game State Store

### Goal

Create the Svelte store that manages all game state, as specified in `frontend_design.md` Section 8.

### Before You Start

Inspect:

- `proof-battle-frontend/src/lib/ws/messages.ts` — the message types from Step 4
- `proof-battle-frontend/src/lib/ws/stateMachine.ts` — the state machine from Step 4
- Svelte store documentation if needed: `writable`, `derived` from `svelte/store`

Confirm:

- Message types are defined
- State machine is implemented
- The store will consume these types

Do not make changes until you have confirmed Steps 4 is complete.

### Context

`frontend_design.md` Section 8 defines `gameStore.ts` as the single source of truth for all game state. It includes `GamePhase`, `GameState` interface, and a factory function returning all mutation methods. This store will be consumed by all game components.

### Implementation Instructions

1. Create `proof-battle-frontend/src/lib/stores/gameStore.ts`:
2. Define `LeanDiagnostic` interface:
   ```ts
   export interface LeanDiagnostic {
     line: number;
     col: number;
     endLine: number;
     endCol: number;
     message: string;
     severity: 'error' | 'warning' | 'info';
   }
   ```
3. Define `GamePhase` type: `'idle' | 'matchmaking' | 'in_game' | 'submitting' | 'won' | 'lost' | 'draw'`
4. Define `GameState` interface with ALL fields from Section 8:
   ```ts
   interface GameState {
     phase: GamePhase;
     playerId: string | null;
     opponentId: string | null;
     challenge: {
       goal: string;
       imports: string[];
       difficulty: number;
       hint: string;
       category: string;
     } | null;
     result: {
       won: boolean;
       winnerProof: string;
       canonicalProof: string;
       eloDelta: number;
     } | null;
     mySubmissions: number;
     opponentActivity: 'idle' | 'typing' | 'submitted';
     lspDiagnostics: LeanDiagnostic[];
     submitting: boolean;
     lastError: string | null;
   }
   ```
5. Implement `createGameStore()` function returning all methods from Section 8:
   - `setOpponent(id)`: sets opponentId, transitions phase to 'matchmaking'
   - `setChallenge(c)`: sets challenge, transitions phase to 'in_game'
   - `setSubmitting(v)`: sets submitting flag, increments mySubmissions if true
   - `setProofRejected(output)`: clears submitting, parses output into lspDiagnostics
   - `setProofAccepted(output)`: clears submitting
   - `setGameEnded(winnerId, myId, extra)`: sets phase to 'won' or 'lost', sets result
   - `setDiagnostics(diags)`: sets lspDiagnostics
   - `reset()`: resets all state to initial values
6. Export: `export const gameStore = createGameStore()`
7. Create derived store: `export const canSubmit = derived(gameStore, $g => $g.phase === 'in_game' && !$g.submitting)`
8. Create a `parseOutput` helper function (used by `setProofRejected`) that parses Lean stderr into `LeanDiagnostic[]`. For now, implement a basic parser that extracts line numbers from error messages like `"line 3, column 5: unknown tactic"`.

### Technical Requirements

- Use Svelte's `writable` and `derived` stores
- The store must be a singleton (created once, exported)
- All state mutations must go through the returned methods (not direct writes)

### Edge Cases

- `setGameEnded` must handle the case where `myId` is null (should not crash)
- `parseOutput` must handle malformed error messages gracefully (return empty array)
- `reset` must return to the exact initial state

### Constraints

- Do not modify any files in `battle/`
- Do not connect to WebSocket yet — only the store logic
- Follow the exact method signatures from Section 8

### Verification

Run:

```bash
cd proof-battle-frontend && npm run check
```

Then verify:

- TypeScript compilation passes
- All store methods are exported and typed
- The derived `canSubmit` store correctly reflects state changes

### Completion Criteria

This step is complete when:

- `src/lib/stores/gameStore.ts` exists with all interfaces and methods
- `canSubmit` derived store is exported
- `npm run check` passes

### Expected Output

At the end of this step, the project should have:

- `src/lib/stores/gameStore.ts` with complete game state management

---

## Step 6 — Implement the WebSocket Client

### Goal

Create the WebSocket client class that connects to the Rust backend, sends messages, and dispatches server messages to the game store, as specified in `frontend_design.md` Section 6.

### Before You Start

Inspect:

- `battle/src/main.rs` — the Axum server handler to understand the connection flow
- `battle/src/message.rs` — the message format the backend expects
- `proof-battle-frontend/src/lib/ws/messages.ts` — the TypeScript types
- `proof-battle-frontend/src/lib/stores/gameStore.ts` — the store to update

Confirm:

- The backend listens on `ws://localhost:3000/ws`
- The backend sends a `Joined` message immediately on connection
- The backend expects `SubmitProof { code, player_id }` (current API)
- The game store methods are ready to receive updates

Do not make changes until you have confirmed Steps 4 and 5 are complete.

### Context

`frontend_design.md` Section 6 defines the `ProofBattleWsClient` class with connect, disconnect, joinQueue, submitProof, and reconnection logic. This client is the bridge between the backend and the frontend state.

### Implementation Instructions

1. Create `proof-battle-frontend/src/lib/ws/client.ts`:
2. Implement `ProofBattleWsClient` class as shown in Section 6:
   - Private fields: `ws`, `reconnectTimer`, `reconnectAttempts`, `MAX_RECONNECTS = 5`
   - Public stores: `state = writable<WsState>('disconnected')`, `playerId = writable<string | null>(null)`
   - `connect(sessionToken?)`: creates WebSocket to `ws://localhost:3000/ws`, handles onopen/onmessage/onclose/onerror
   - `disconnect()`: clears reconnect timer, closes WebSocket with code 1000
   - `joinQueue()`: sends `{ type: 'Join', username: '...' }` (ASSUMPTION: use a stored username — create a simple `username` field on the client for now)
   - `submitProof(code)`: sends `{ type: 'SubmitProof', code, player_id }` where `player_id` is read from `this.playerId` store
   - `handleServerMessage(msg)`: switch on `msg.type`, call appropriate `gameStore` methods
   - Reconnect: exponential backoff (1s, 2s, 4s, 8s, 16s, 30s max), max 5 attempts
3. Export as singleton: `export const wsClient = new ProofBattleWsClient()`

### Implementation Details for `handleServerMessage`

Based on the current backend's `ServerMessage` types:
- `Joined`: set `playerId` store, transition state
- `MatchFound`: call `gameStore.setOpponent(msg.opponent)`
- `Challenge`: call `gameStore.setChallenge({ ...msg, difficulty: 1, hint: '', category: '' })` (backend doesn't send difficulty/hint/category yet — use defaults)
- `ProofResult`: if success, call `gameStore.setProofAccepted(msg.output)`; if failure, call `gameStore.setProofRejected(msg.output)`
- `GameEnded`: call `gameStore.setGameEnded(msg.winner, get(playerId), {})`
- `Error`: log to console, set `lastError` on game store

### Technical Requirements

- Singleton pattern (one WebSocket per browser tab)
- The client must use the current backend's message format (with `player_id` in SubmitProof)
- Reconnection must use exponential backoff
- The client must handle disconnection gracefully (update state machine)

### Edge Cases

- If the WebSocket URL is wrong, the client should retry with backoff
- If the server sends an unknown message type, log a warning but do not crash
- If `playerId` is null when `submitProof` is called, return early without sending

### Constraints

- Do not modify any files in `battle/`
- Do not implement session token reconnection yet (backend doesn't support it)
- The `joinQueue` method should send a `Join` message (current backend format)

### Verification

Run:

```bash
cd proof-battle-frontend && npm run check
```

Then verify:

- TypeScript compilation passes
- The client can be imported and instantiated
- The singleton export works

### Completion Criteria

This step is complete when:

- `src/lib/ws/client.ts` exists with the full ProofBattleWsClient class
- The singleton is exported
- `npm run check` passes

### Expected Output

At the end of this step, the project should have:

- `src/lib/ws/client.ts` with WebSocket client ready to connect to the backend

---

## Step 7 — Build the Monaco Editor Component

### Goal

Create the Monaco Editor wrapper component with Lean 4 syntax highlighting, custom theme, and Unicode input, as specified in `frontend_design.md` Section 4.

### Before You Start

Inspect:

- `proof-battle-frontend/package.json` — confirm `@monaco-editor/loader` is installed
- `frontend_design.md` Section 4.2, 4.3, 4.4, 4.5 — the component code, grammar, theme, and Unicode mapper

Confirm:

- Monaco Editor dependency is installed
- Design tokens (Step 2) are available for the editor theme
- The editor will be used in the game screen (Step 10)

Do not make changes until you have confirmed the dependency exists.

### Context

`frontend_design.md` Section 4 explains why Monaco was chosen (VS Code engine, LSP support, Lean extension compatibility) and provides the complete component code, Monarch grammar, theme, and Unicode mapper. This is the core editor experience.

### Implementation Instructions

1. Create `proof-battle-frontend/src/lib/editor/lean4Grammar.ts`:
   - Export `getLean4Grammar()` returning `Monaco.languages.IMonarchLanguage`
   - Include ALL keywords, typeKeywords, operators, and tokenizer rules from Section 4.3
   - Block comment support: `/- ... -/` syntax

2. Create `proof-battle-frontend/src/lib/editor/editorTheme.ts`:
   - Export `getEditorTheme()` returning `Monaco.editor.IStandaloneThemeData`
   - Use exact color values from Section 4.4 (editor bg `#111113`, cursor `#7c6af7`, etc.)

3. Create `proof-battle-frontend/src/lib/editor/unicodeMapper.ts`:
   - Export `installUnicodeMapper(editor)` function from Section 4.5
   - Include ALL Unicode mappings from the `LEAN_UNICODE_MAP` in Section 4.5
   - On Space/Tab keypress, check for `\keyword` pattern, replace with Unicode

4. Create `proof-battle-frontend/src/lib/editor/MonacoEditor.svelte`:
   - Props: `value: string = ''`, `readonly: boolean = false`, `diagnostics: LeanDiagnostic[] = []`
   - Events: dispatch('change', code) on content change
   - onMount: lazy-load Monaco via `@monaco-editor/loader`
   - Register 'lean4' language, set Monarch tokenizer, set theme
   - Configure editor options from Section 4.2: fontSize 14, JetBrains Mono, lineNumbers on, minimap off, wordWrap on, bracketPairColorization on, cursorBlinking smooth, padding top/bottom 16
   - Call `installUnicodeMapper(editor)` after creation
   - Implement `applyDiagnostics` function from Section 4.2 that converts `LeanDiagnostic[]` to Monaco markers
   - Reactively update diagnostics when the prop changes: `$: if (editor && diagnostics) { applyDiagnostics(diagnostics); }`
   - Export `getValue()` and `insertSnippet(text)` methods
   - Container div with `min-height: 400px`

### Technical Requirements

- Monaco loaded lazily (4MB bundle, loaded on game screen mount)
- Lean 4 Monarch grammar must handle all Unicode operators (∀, ∃, →, ↔, ∧, ∨, ¬, ⊢, etc.)
- The editor theme must match the design token colors
- Unicode input must work identically to VS Code's Lean extension

### Edge Cases

- If Monaco fails to load (network issue), show a fallback textarea with a warning
- If the grammar encounters an unrecognized token, fall back to 'identifier' style
- Unicode mapper should not fire on backspace (only Space/Tab)

### Constraints

- Do not modify any files in `battle/`
- Do not implement LSP streaming yet — only static editor with diagnostics prop
- Do not connect to the WebSocket yet — the editor is a standalone component

### Verification

Run:

```bash
cd proof-battle-frontend && npm run check
```

Then verify:

- TypeScript compilation passes
- The component can be imported and rendered
- Monaco loads lazily (check Network tab — Monaco resources should not load until the component mounts)

### Completion Criteria

This step is complete when:

- `src/lib/editor/MonacoEditor.svelte` exists with full Monaco integration
- `src/lib/editor/lean4Grammar.ts` has complete Lean 4 grammar
- `src/lib/editor/editorTheme.ts` has the dark theme
- `src/lib/editor/unicodeMapper.ts` has all Unicode mappings
- `npm run check` passes

### Expected Output

At the end of this step, the project should have:

- Four files in `src/lib/editor/` providing the complete Monaco editor experience

---

## Step 8 — Build the Problem Panel and KaTeX Rendering

### Goal

Create the Problem Panel component that displays the proof goal with KaTeX rendering, difficulty stars, and hints, as specified in `frontend_design.md` Section 5.

### Before You Start

Inspect:

- `proof-battle-frontend/package.json` — confirm `katex` is installed
- `frontend_design.md` Section 5 — the ProblemPanel.svelte code

Confirm:

- KaTeX dependency is installed
- Design tokens are available
- The component will receive `goal` from the game store

Do not make changes until you have confirmed the dependency.

### Context

`frontend_design.md` Section 5 provides the complete ProblemPanel.svelte code. The component renders the Lean proof goal as KaTeX math, shows difficulty stars, and provides a toggleable hint.

### Implementation Instructions

1. Create `proof-battle-frontend/src/lib/components/game/ProblemPanel.svelte`:
2. Implement exactly as shown in Section 5:
   - Props: `goal: string`, `difficulty: number`, `category: string`, `hint: string`, `showHint: boolean`
   - `leanToKaTeX` function: convert Lean Unicode to LaTeX (all replacements from Section 5)
   - Render with `katex.renderToString(leanToKaTeX(goal), { throwOnError: false, displayMode: true })`
   - Difficulty stars: 5 stars, filled based on difficulty value
   - Hint toggle: button shows/hides hint text with `transition:slide`
   - Display raw Lean syntax below rendered KaTeX
3. Style with design tokens: `.panel` background, `var(--font-mono)` for goal, `var(--text-primary)` for rendered math

### Technical Requirements

- KaTeX rendering must not throw on invalid LaTeX (use `throwOnError: false`)
- The `leanToKaTeX` function must handle all Unicode symbols from the design doc
- Difficulty stars must render exactly 5, with `filled` class for active stars

### Edge Cases

- If `goal` is empty, show nothing (no error)
- If KaTeX rendering fails, fall back to raw `<code>` display
- If `hint` is empty, hide the hint section entirely

### Constraints

- Do not modify any files in `battle/`
- Do not connect to game store yet — this is a standalone component with props

### Verification

Run:

```bash
cd proof-battle-frontend && npm run check
```

Then verify:

- TypeScript compilation passes
- The component can be imported and rendered with test props

### Completion Criteria

This step is complete when:

- `src/lib/components/game/ProblemPanel.svelte` exists with KaTeX rendering
- `npm run check` passes

### Expected Output

At the end of this step, the project should have:

- `src/lib/components/game/ProblemPanel.svelte` ready for use in the game screen

---

## Step 9 — Build the Status Panel, Timer, and UI Components

### Goal

Create the Status Panel, Timer, Spinner, and Badge components as specified in `frontend_design.md` Sections 5 and 10.

### Before You Start

Inspect:

- `frontend_design.md` Section 3.3 — the game screen wireframe showing the Status Panel
- `frontend_design.md` Section 10 — Timer, SubmitButton, and confetti details

Confirm:

- Design tokens are available
- These components will be used in the game screen (Step 10)

Do not make changes until you have confirmed the design details.

### Context

The game screen has three panels. The rightmost panel (220px) shows player status, opponent activity, and submission counts. The Timer shows countdown urgency states. These are supporting components for the game screen.

### Implementation Instructions

1. Create `proof-battle-frontend/src/lib/components/game/StatusPanel.svelte`:
   - Two sections: "YOU" and "OPPONENT"
   - Each shows activity status: idle / typing... / checking... / submitted
   - Shows submission count for each player
   - Props: `myStatus`, `opponentStatus`, `mySubmissions`, `opponentSubmissions`
   - Style as right sidebar with `var(--bg-surface)` background

2. Create `proof-battle-frontend/src/lib/ui/Timer.svelte`:
   - Props: `seconds: number`
   - Computed: minutes, secs, urgent (<=60), critical (<=10)
   - Display: `⏱ MM:SS`
   - CSS classes: `.urgent` uses `var(--warning)`, `.critical` uses `var(--error)` with pulse animation
   - Font: `font-variant-numeric: tabular-nums`

3. Create `proof-battle-frontend/src/lib/ui/Spinner.svelte`:
   - CSS-only spinning animation (no external deps)
   - Props: `size: number` (default 16)

4. Create `proof-battle-frontend/src/lib/ui/Badge.svelte`:
   - Props: `variant: 'difficulty' | 'category'`, `value: string | number`
   - Styled with `var(--accent-dim)` background, `var(--accent)` text

5. Create `proof-battle-frontend/src/lib/components/game/SubmitButton.svelte`:
   - Props: `submitting: boolean`, `disabled: boolean`, `lastResult: 'success' | 'error' | null`
   - States from Section 10:
     - Default: "Submit Proof →"
     - Submitting: Spinner + "Verifying with Lean..."
     - Success: "Proof accepted!" (green background)
     - Error: "Try again" (red background)
   - CSS pulse animation during submission

### Technical Requirements

- All components use design tokens (no hardcoded colors)
- Timer must handle 0 seconds gracefully
- Spinner must be pure CSS (no JS animation)

### Edge Cases

- Timer at 0 seconds should show "0:00" without negative values
- SubmitButton should not be clickable when disabled or submitting
- StatusPanel should handle null/undefined status gracefully

### Constraints

- Do not modify any files in `battle/`
- Do not connect to stores yet — these are prop-driven components

### Verification

Run:

```bash
cd proof-battle-frontend && npm run check
```

Then verify:

- TypeScript compilation passes
- All components can be imported and rendered

### Completion Criteria

This step is complete when:

- `src/lib/components/game/StatusPanel.svelte` exists
- `src/lib/ui/Timer.svelte` exists
- `src/lib/ui/Spinner.svelte` exists
- `src/lib/ui/Badge.svelte` exists
- `src/lib/components/game/SubmitButton.svelte` exists
- `npm run check` passes

### Expected Output

At the end of this step, the project should have:

- Five new UI components ready for the game screen

---

## Step 10 — Build the Result Modal

### Goal

Create the Result Modal component that shows win/lose state with confetti, as specified in `frontend_design.md` Sections 3.4 and 10.

### Before You Start

Inspect:

- `proof-battle-frontend/package.json` — confirm `canvas-confetti` is installed
- `frontend_design.md` Section 3.4 — the ResultModal wireframe
- `frontend_design.md` Section 10 — confetti configuration

Confirm:

- canvas-confetti dependency is installed
- Game store has the `result` field structure

Do not make changes until you have confirmed the dependency.

### Context

The Result Modal appears when a game ends. It shows the winner, time, ELO change, winning proof, and canonical solution. Confetti fires on win.

### Implementation Instructions

1. Create `proof-battle-frontend/src/lib/components/game/ResultModal.svelte`:
2. Props: `result: { won: boolean, winnerProof: string, canonicalProof: string, eloDelta: number }`, `timeElapsed: number`
3. When `won === true`:
   - Display "YOU WIN!" with trophy emoji (only if user explicitly requested — otherwise use text "WIN")
   - Show time as `Xm Ys`
   - Show ELO delta: `+N rating points`
   - Show winning proof in a `<pre><code>` block
   - Show canonical solution in a separate `<pre><code>` block
   - On mount, trigger confetti:
     ```ts
     import confetti from 'canvas-confetti';
     confetti({ particleCount: 150, spread: 80, origin: { y: 0.6 }, colors: ['#7c6af7', '#34d399', '#fbbf24'] });
     ```
4. When `won === false`:
   - Display "YOU LOST" with muted styling
   - Same layout but no confetti
5. Two buttons: "Rematch" and "New Opponent"
   - Dispatch events: `on:rematch`, `on:newOpponent`
6. Modal overlay: dark backdrop (`rgba(0,0,0,0.7)`), centered card with `var(--bg-elevated)` background

### Technical Requirements

- Confetti must only fire when `won === true` and component mounts
- The modal must be dismissible (click backdrop to close)
- Time formatting must handle 0 seconds and > 60 seconds correctly

### Edge Cases

- If `result` is null, render nothing
- If `winnerProof` or `canonicalProof` is empty, show "Not available"
- If `eloDelta` is negative, show with minus sign

### Constraints

- Do not modify any files in `battle/`
- Do not connect to stores yet — prop-driven component

### Verification

Run:

```bash
cd proof-battle-frontend && npm run check
```

Then verify:

- TypeScript compilation passes
- Component can be imported and rendered with test props

### Completion Criteria

This step is complete when:

- `src/lib/components/game/ResultModal.svelte` exists with confetti
- `npm run check` passes

### Expected Output

At the end of this step, the project should have:

- `src/lib/components/game/ResultModal.svelte` ready for the game screen

---

## Step 11 — Assemble the Game Screen

### Goal

Wire all components together into the full game screen, connecting the WebSocket client, game store, and all UI components, as specified in `frontend_design.md` Sections 3.3, 5, and 10.

### Before You Start

Inspect:

- `proof-battle-frontend/src/lib/ws/client.ts` — the WebSocket client
- `proof-battle-frontend/src/lib/stores/gameStore.ts` — the game store
- `proof-battle-frontend/src/lib/editor/MonacoEditor.svelte` — the editor component
- `proof-battle-frontend/src/lib/components/game/ProblemPanel.svelte` — the problem panel
- `proof-battle-frontend/src/lib/components/game/StatusPanel.svelte` — the status panel
- `proof-battle-frontend/src/lib/components/game/ResultModal.svelte` — the result modal
- `proof-battle-frontend/src/lib/components/game/SubmitButton.svelte` — the submit button
- `proof-battle-frontend/src/lib/ui/Timer.svelte` — the timer

Confirm:

- All components from Steps 7-10 exist and compile
- The WebSocket client is ready to connect
- The game store is ready to receive updates

Do not make changes until you have confirmed all previous components exist.

### Context

This is the integration step where all pieces come together. The game screen is described in `frontend_design.md` Section 3.3 as a three-panel layout with header, problem panel, editor, status panel, and result modal.

### Implementation Instructions

1. Read the existing `proof-battle-frontend/src/routes/game/+page.svelte` (placeholder from Step 3).
2. Replace it with the full game screen from Section 5 (`GameScreen.svelte` equivalent):
   - **Header bar**: ProofBattle logo (left), player names + ELO (center), timer (right)
   - **Three-column grid**: ProblemPanel (280px) | Editor + SubmitButton (flex) | StatusPanel (220px)
   - **Mobile responsive**: `@media (max-width: 900px)` — stack to single column
3. Wire up WebSocket:
   - `onMount`: call `wsClient.connect()`
   - `onDestroy`: call `wsClient.disconnect()`
4. Wire up editor:
   - Bind to MonacoEditor component ref
   - On submit: get code from editor, call `wsClient.submitProof(code)`
   - Disable submit when `canSubmit` is false
5. Wire up store subscriptions:
   - Pass `gameStore.challenge` to ProblemPanel
   - Pass `gameStore.lspDiagnostics` to MonacoEditor
   - Pass `gameStore.opponentActivity` to StatusPanel
   - Pass `gameStore.result` to ResultModal
6. Show ResultModal when `gameStore.result` is non-null
7. Pass Timer the remaining seconds (ASSUMPTION: use a local countdown timer set to 600 seconds for now, since the backend has no timer)

### Technical Requirements

- The layout must use CSS Grid as specified in Section 3.3
- Monaco editor must be the center panel (takes remaining space)
- The timer must be in the header
- The submit button must be below the editor

### Edge Cases

- If the WebSocket disconnects, show a reconnection indicator
- If the challenge is null (before match), show a loading state
- If the editor ref is null, do not attempt to get code

### Constraints

- Do not modify any files in `battle/`
- Do not add new dependencies
- Preserve all existing component interfaces

### Verification

Run:

```bash
cd proof-battle-frontend && npm run dev
```

Then verify:

- Navigate to `/game` — the full layout renders
- The Monaco editor loads with Lean syntax highlighting
- The ProblemPanel shows a placeholder goal (empty if no challenge)
- `npm run check` passes

### Completion Criteria

This step is complete when:

- `/game` shows the three-panel layout
- All components are wired to the store
- The WebSocket client connects on mount
- `npm run check` passes

### Expected Output

At the end of this step, the project should have:

- A complete game screen at `/game` with all components integrated

---

## Step 12 — Build the Landing Page

### Goal

Implement the landing page with username input, Play Now button, and stats display, as specified in `frontend_design.md` Section 3.1.

### Before You Start

Inspect:

- `proof-battle-frontend/src/routes/+page.svelte` — the current placeholder
- `frontend_design.md` Section 3.1 — the landing page wireframe

Confirm:

- The shared layout renders on the landing page
- Design tokens are available

Do not make changes until you have confirmed the layout works.

### Context

The landing page is the entry point. It shows the title, username input, Play Now / Practice buttons, and live stats. The username is stored for use in the matchmaking flow.

### Implementation Instructions

1. Read the existing `proof-battle-frontend/src/routes/+page.svelte` (placeholder from Step 3).
2. Replace it with the landing page from Section 3.1:
   - Centered vertically and horizontally
   - Title: "⊢ ProofBattle" (large, `var(--font-mono)`, `var(--text-primary)`)
   - Subtitle: "Competitive Lean Theorem Proving" (`var(--text-secondary)`)
   - Username input field (styled with `var(--bg-elevated)` background, `var(--bg-border)` border, `var(--radius-md)` radius)
   - Two buttons: "Play Now" (`btn-primary`) and "Practice" (secondary style)
   - Bottom stats bar: "Live Games: 12    Players Online: 47    Top ELO: 2140" (hardcoded)
3. Create `proof-battle-frontend/src/lib/stores/userStore.ts`:
   - `export const userStore = writable<{ username: string | null }>({ username: null })`
   - On "Play Now" click: set username, navigate to `/lobby`
4. "Practice" button: navigate to `/lobby` (ASSUMPTION: practice mode uses the same flow as matchmaking for now)
5. Style the input and buttons using design tokens

### Technical Requirements

- Use SvelteKit's `goto` for navigation (import from `$app/navigation`)
- Username must be non-empty to enable "Play Now"
- Stats are hardcoded (no server endpoint)

### Edge Cases

- If username is empty, disable the "Play Now" button
- If user navigates directly to `/lobby` without entering username, redirect to `/`

### Constraints

- Do not modify any files in `battle/`
- Do not add new dependencies

### Verification

Run:

```bash
cd proof-battle-frontend && npm run dev
```

Then verify:

- Navigate to `/` — landing page renders with title, input, buttons
- Enter username, click "Play Now" — navigates to `/lobby`
- `npm run check` passes

### Completion Criteria

This step is complete when:

- Landing page renders with all elements from Section 3.1
- Username is stored in userStore
- Navigation to `/lobby` works
- `npm run check` passes

### Expected Output

At the end of this step, the project should have:

- Complete landing page at `/`
- `src/lib/stores/userStore.ts` with username state

---

## Step 13 — Build the Lobby / Matchmaking Screen

### Goal

Implement the matchmaking screen with animation, WebSocket queue entry, and navigation to the game, as specified in `frontend_design.md` Section 3.2.

### Before You Start

Inspect:

- `proof-battle-frontend/src/routes/lobby/+page.svelte` — the current placeholder
- `frontend_design.md` Section 3.2 — the matchmaking wireframe
- `proof-battle-frontend/src/lib/ws/client.ts` — the joinQueue method
- `proof-battle-frontend/src/lib/stores/gameStore.ts` — the phase transitions

Confirm:

- The WebSocket client has a `joinQueue` method
- The game store transitions to `in_game` when a challenge arrives
- The username is stored (from Step 12)

Do not make changes until you have confirmed the WebSocket client and store are ready.

### Context

The lobby screen shows the matchmaking animation while waiting for an opponent. When a match is found, the game store transitions to `in_game` and the user navigates to `/game`.

### Implementation Instructions

1. Read the existing `proof-battle-frontend/src/routes/lobby/+page.svelte` (placeholder from Step 3).
2. Replace it with the matchmaking screen from Section 3.2:
   - Centered layout
   - "Finding your opponent..." text
   - Three animated pulse dots (CSS keyframes with staggered `animation-delay`)
   - "Your ELO: 1024 — searching ±150 range" (static)
   - Cancel button that navigates back to `/`
3. On mount:
   - Read username from `userStore`
   - If no username, redirect to `/`
   - Call `wsClient.joinQueue(username)` (or `wsClient.connect()` then `wsClient.joinQueue()`)
   - Subscribe to `gameStore.phase`:
     - If phase becomes `'in_game'`: navigate to `/game`
4. Cancel button: call `wsClient.disconnect()`, navigate to `/`
5. CSS animation for pulse dots:
   ```css
   @keyframes pulse {
     0%, 100% { opacity: 0.3; }
     50% { opacity: 1; }
   }
   ```
   Three dots with `animation-delay: 0s`, `0.2s`, `0.4s`

### Technical Requirements

- Must check for username before joining queue
- Must clean up WebSocket on unmount
- Animation must be CSS-only (no JS timers)

### Edge Cases

- If user navigates to `/lobby` without a username, redirect to `/`
- If WebSocket fails to connect, show error message
- If user clicks Cancel, clean up properly

### Constraints

- Do not modify any files in `battle/`
- Do not add new dependencies

### Verification

Run:

```bash
cd proof-battle-frontend && npm run dev
```

Then verify:

- Navigate to `/lobby` with username set — animation plays
- Cancel button returns to `/`
- `npm run check` passes

### Completion Criteria

This step is complete when:

- Lobby screen shows matchmaking animation
- WebSocket queue entry works
- Cancel cleans up properly
- `npm run check` passes

### Expected Output

At the end of this step, the project should have:

- Complete lobby screen at `/lobby`

---

## Step 14 — Build the Diagnostics Bar

### Goal

Create the Diagnostics Bar component that shows Lean error messages below the editor, as specified in `frontend_design.md` Section 9.

### Before You Start

Inspect:

- `proof-battle-frontend/src/lib/stores/gameStore.ts` — the `lspDiagnostics` field
- `frontend_design.md` Section 9 — the diagnostics approach

Confirm:

- The game store has `lspDiagnostics: LeanDiagnostic[]`
- The game screen (Step 11) already passes diagnostics to the editor

Do not make changes until you have confirmed the store and game screen are ready.

### Context

The Diagnostics Bar shows Lean error messages below the editor. It is collapsible and styled with the design tokens. Section 9 describes two approaches — Approach A (submit on debounce) is the recommended first implementation.

### Implementation Instructions

1. Create `proof-battle-frontend/src/lib/components/game/DiagnosticsBar.svelte`:
   - Positioned below the editor in the game screen
   - Shows the latest diagnostic from `gameStore.lspDiagnostics`
   - Display format: `⚠ Line {line}: {message}`
   - Color: `var(--error)` for errors, `var(--warning)` for warnings
   - Collapsible: click to expand/collapse full diagnostics list
   - Style: `var(--bg-elevated)` background, `var(--bg-border)` top border

2. Create `proof-battle-frontend/src/lib/utils/debounce.ts`:
   - Export `debounce<T>(fn: T, ms: number): T` utility function

3. Update `proof-battle-frontend/src/routes/game/+page.svelte`:
   - Add DiagnosticsBar below the Monaco Editor, above the SubmitButton
   - Pass `gameStore.lspDiagnostics` to it

4. Debounced diagnostics (placeholder):
   - When editor content changes, debounce at 800ms
   - For now, this is a no-op (the actual LSP streaming requires backend changes)
   - Add a comment: `// TODO: Send CheckProof message when backend implements it`

### Technical Requirements

- DiagnosticsBar must be collapsible (click to toggle)
- The debounce utility must be generic and reusable
- The bar must not show if there are no diagnostics

### Edge Cases

- If `lspDiagnostics` is empty, render nothing
- If the last diagnostic is a warning (not error), use warning color
- If diagnostics list is long, show scrollable list in expanded state

### Constraints

- Do not modify any files in `battle/`
- Do not add new dependencies
- Do not implement actual LSP streaming — only the UI shell

### Verification

Run:

```bash
cd proof-battle-frontend && npm run check
```

Then verify:

- TypeScript compilation passes
- The DiagnosticsBar can be imported and rendered

### Completion Criteria

This step is complete when:

- `src/lib/components/game/DiagnosticsBar.svelte` exists
- `src/lib/utils/debounce.ts` exists
- The game screen includes the DiagnosticsBar
- `npm run check` passes

### Expected Output

At the end of this step, the project should have:

- DiagnosticsBar component integrated into the game screen
- Debounce utility available for future use

---

## Step 15 — Full Integration Review

### Goal

Verify that the entire frontend implementation matches the design document, fix any issues, and produce an implementation report.

### Before You Start

Inspect:

- All files created in Steps 1-14
- `frontend_design.md` — the original design document
- `battle/src/message.rs` — the backend API contract

Confirm:

- All components exist and compile
- The WebSocket client connects to the backend
- The game flow works end-to-end (landing → lobby → game → result)

Do not make changes until you have verified the current state.

### Context

This is the final integration step. The coding agent must review all changes, verify design fidelity, and fix any issues.

### Implementation Instructions

1. **Review all created files**:
   - Verify each component matches the design doc's specifications
   - Verify all CSS custom properties are used (no hardcoded colors)
   - Verify all message types are handled

2. **Run the complete test suite**:
   ```bash
   cd proof-battle-frontend
   npm run check        # TypeScript compilation
   npm run lint         # ESLint
   npm run format       # Prettier
   npm run build        # Production build
   npm run test         # Vitest unit tests
   ```

3. **Fix any issues found**:
   - TypeScript errors
   - Lint warnings
   - Build failures
   - Missing imports
   - Unused variables

4. **Verify design fidelity against `frontend_design.md`**:
   - Section 1 (Tech Stack): SvelteKit, Monaco, KaTeX, Tailwind — all used
   - Section 2 (Design System): all tokens defined and applied
   - Section 3 (Screens): Landing, Lobby, Game all implemented
   - Section 4 (Monaco): grammar, theme, Unicode mapper all implemented
   - Section 5 (Components): ProblemPanel, StatusPanel, etc. all implemented
   - Section 6 (WebSocket): client and state machine implemented
   - Section 7 (Message Contract): types defined matching design doc
   - Section 8 (State Management): gameStore implemented
   - Section 9 (Diagnostics): DiagnosticsBar implemented
   - Section 10 (Performance/UX): Monaco lazy loading, button states, timer implemented
   - Section 11 (File Structure): directory structure matches design

5. **Identify missing requirements**:
   - Session token reconnection (backend doesn't support it yet)
   - LSP streaming (backend doesn't support it yet)
   - Opponent activity indicators (backend doesn't send them yet)
   - Timer driven by server (backend doesn't send timer updates)
   - Dynamic stats on landing page (no server endpoint)
   - Practice mode (not designed)

6. **Produce implementation report** listing:
   - Features implemented
   - Files created/modified
   - Tests added
   - Known limitations
   - Assumptions made
   - Remaining open questions

### Technical Requirements

- All existing tests must pass
- Build must succeed
- No TypeScript errors
- No lint errors

### Edge Cases

- If the backend is not running, the frontend should not crash — show connection error
- If the backend sends unexpected message types, handle gracefully

### Constraints

- Do not modify any files in `battle/`
- Do not refactor unrelated working code
- Do not add features not in the design doc

### Verification

Run:

```bash
cd proof-battle-frontend
npm run check && npm run lint && npm run build && npm run test
```

Then verify:

- All commands succeed
- No errors in the output
- The production build completes

### Completion Criteria

This step is complete when:

- All tests pass
- Build succeeds
- Design fidelity is verified
- Implementation report is produced

### Expected Output

At the end of this step, the project should have:

- A complete, working ProofBattle frontend that matches the design document
- All components, stores, and utilities implemented
- A clean build with no errors
