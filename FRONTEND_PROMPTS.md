# ProofBattle — Frontend Implementation Prompts

> **Source of truth**: `battle/frontend_design.md` and the message contract with the Rust backend
> These prompts are ordered by dependency. Step N cannot start until Step N-1 is complete.
> Every instruction below is grounded in the design document — no hallucinated APIs or features.

---

## Step 1: Scaffold the SvelteKit Project

**Prompt:**

```
Create a new SvelteKit project for the ProofBattle frontend. Do NOT use an existing frontend
directory if one exists — start fresh in a new directory called proof-battle-frontend at the
repository root.

Requirements:

1. Initialize with: npm create svelte@latest proof-battle-frontend
   - Template: Skeleton project
   - TypeScript: Yes
   - ESLint: Yes
   - Prettier: Yes
   - Playwright: Yes
   - Vitest: Yes

2. Install dependencies:
   npm install @monaco-editor/loader katex lucide-svelte canvas-confetti
   npm install -D tailwindcss @tailwindcss/vite

3. Configure Tailwind CSS v4:
   - Add the Tailwind Vite plugin to vite.config.ts.
   - Create src/app.css with Tailwind imports and the design tokens from Section 2.1
     (all CSS custom properties under :root for bg-base, bg-surface, accent, etc.).

4. Add font links to src/app.html:
   - JetBrains Mono (for code): https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;600;700
   - Inter (for UI): https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700
   - KaTeX CSS: https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/katex.min.css

5. Verify the project runs: npm run dev should start without errors.
```

---

## Step 2: Create the Design Token System and Global Styles

**Prompt:**

```
Implement the complete design token system from frontend_design.md Section 2.

Requirements:

1. In src/app.css (or a dedicated src/lib/styles/tokens.css imported by app.css), define ALL
   CSS custom properties from Section 2.1:
   - Backgrounds: --bg-base, --bg-surface, --bg-elevated, --bg-overlay, --bg-border
   - Text: --text-primary, --text-secondary, --text-muted
   - Accent: --accent, --accent-dim, --accent-bright
   - Semantic: --success, --error, --warning, --info
   - Editor: --editor-bg, --editor-gutter, --editor-line-hl, --editor-cursor
   - Fonts: --font-mono, --font-ui
   - Spacing: --space-1 through --space-16 (8pt grid from Section 2.3)
   - Border radius: --radius-sm, --radius-md, --radius-lg, --radius-xl

2. Define base typography styles:
   - body: font-family var(--font-ui), background var(--bg-base), color var(--text-primary)
   - code/pre: font-family var(--font-mono)

3. Define reusable utility classes:
   - .panel (from Section 2.4: bg-surface, border, radius-lg)
   - .btn-primary (from Section 2.4: accent bg, white text, padding, radius-md, hover/active states)
   - .status-label (from Section 2.2: font-ui, 0.75rem, uppercase, letter-spacing)

4. Set up Tailwind theme extension to use these CSS variables so both systems coexist.

5. Verify: create a test page at src/routes/+page.svelte that renders a .panel containing a
   .btn-primary and .status-label. Confirm styles apply correctly in the browser.
```

---

## Step 3: Implement the SvelteKit Route Structure

**Prompt:**

```
Create the route structure from frontend_design.md Section 11 (File Structure).

Requirements:

1. Create these route files:
   - src/routes/+layout.svelte       — global layout (nav bar, global providers)
   - src/routes/+page.svelte         — landing page (placeholder content for now)
   - src/routes/lobby/+page.svelte   — matchmaking screen (placeholder)
   - src/routes/game/+page.svelte    — game screen (placeholder)

2. In +layout.svelte:
   - Import src/app.css
   - Add a minimal nav bar with the ProofBattle logo/text: "⊢ ProofBattle"
   - Use a <slot /> for child routes

3. In each placeholder page, render a heading that identifies the page:
   - Landing: "ProofBattle — Competitive Lean Theorem Proving"
   - Lobby: "Finding your opponent..."
   - Game: "Game Screen"

4. Verify all three routes load: /, /lobby, /game.

5. The actual page content for each route will be implemented in later steps.
```

---

## Step 4: Build the WebSocket Client Class and State Machine

**Prompt:**

```
Implement the WebSocket client and state machine from frontend_design.md Sections 6 and 7.

Requirements:

1. Create src/lib/ws/stateMachine.ts:
   - Define WsState type with all states: disconnected, connecting, connected_idle, matchmaking,
     game_starting, in_game, submitting, game_ended, error (from Section 6).
   - Define WsEvent type with all event types (from Section 6).
   - Define the TRANSITIONS record mapping (state, event) → new_state (from Section 6).
   - Implement a function: transition(currentState: WsState, event: WsEvent): WsState
     that returns the new state or throws if the transition is invalid.

2. Create src/lib/ws/messages.ts:
   - Define ClientMessage type as a discriminated union:
     { type: 'JoinQueue' } | { type: 'SubmitProof'; code: string } | { type: 'Ping' }
   - Define ServerMessage type as a discriminated union with ALL message types from Section 7:
     Joined, MatchFound, Challenge, ProofResult, GameEnded, OpponentActivity, TimerUpdate,
     LspDiagnostic, SessionToken, Error, Pong
   - Include ALL fields as specified in the design doc (e.g., Challenge includes goal, imports,
     difficulty, hint, category; GameEnded includes winner, winning_proof, canonical_proof, elo_delta).

3. Create src/lib/ws/client.ts:
   - Implement ProofBattleWsClient class as shown in Section 6.
   - connect(): creates WebSocket to ws://localhost:3000/ws, handles onopen/onmessage/onclose/onerror.
   - disconnect(): closes WebSocket with code 1000.
   - joinQueue(): sends { type: 'JoinQueue' }.
   - submitProof(code): sends { type: 'SubmitProof', code } (NO player_id — server knows).
   - handleServerMessage(): dispatches each ServerMessage type to the appropriate gameStore method.
   - Reconnect logic: exponential backoff (1s, 2s, 4s, 8s, 16s, 30s max), max 5 attempts.
   - Export as singleton: export const wsClient = new ProofBattleWsClient()

4. For now, have client.ts import from gameStore (which doesn't exist yet — create a minimal
   placeholder at src/lib/stores/gameStore.ts with just a writable store and empty methods).

5. Verify: npm run check passes (TypeScript compilation).
```

---

## Step 5: Implement the Game Store

**Prompt:**

```
Implement the game state store from frontend_design.md Section 8.

Requirements:

1. Create src/lib/stores/gameStore.ts (replace the placeholder from Step 4).

2. Define GamePhase type: 'idle' | 'matchmaking' | 'in_game' | 'submitting' | 'won' | 'lost' | 'draw'

3. Define GameState interface with ALL fields from Section 8:
   - phase: GamePhase
   - playerId: string | null
   - opponentId: string | null
   - challenge: { goal, imports, difficulty, hint, category } | null
   - result: { won, winnerProof, canonicalProof, eloDelta } | null
   - mySubmissions: number
   - opponentActivity: 'idle' | 'typing' | 'submitted'
   - lspDiagnostics: LeanDiagnostic[]
   - submitting: boolean
   - lastError: string | null

4. Define LeanDiagnostic interface:
   - line: number, col: number, endLine: number, endCol: number
   - message: string, severity: 'error' | 'warning' | 'info'

5. Implement createGameStore function returning ALL methods from Section 8:
   - setOpponent(id), setChallenge(c), setSubmitting(v)
   - setProofRejected(output), setProofAccepted(output)
   - setGameEnded(winnerId, myId, extra), setDiagnostics(diags), reset()

6. Export: export const gameStore = createGameStore()

7. Create derived store: export const canSubmit = derived(gameStore, $g => $g.phase === 'in_game' && !$g.submitting)

8. Verify: npm run check passes.
```

---

## Step 6: Build the Monaco Editor Component

**Prompt:**

```
Implement the Monaco Editor wrapper component from frontend_design.md Section 4.

Requirements:

1. Create src/lib/editor/MonacoEditor.svelte as shown in Section 4.2:
   - Props: value (string), readonly (boolean), diagnostics (LeanDiagnostic[])
   - Events: dispatch('change', code) on content change
   - onMount: lazy-load Monaco via @monaco-editor/loader
   - Register 'lean4' language
   - Set Monarch tokenizer via getLean4Grammar()
   - Set theme via getEditorTheme()
   - Configure editor options: fontSize 14, JetBrains Mono, lineNumbers on, minimap off,
     wordWrap on, bracketPairColorization on, cursorBlinking smooth, padding top/bottom 16
   - createDecorationsCollection for diagnostics
   - Export getValue() and insertSnippet(text) methods

2. Create src/lib/editor/lean4Grammar.ts as shown in Section 4.3:
   - Export getLean4Grammar() returning a Monaco.languages.IMonarchLanguage
   - Include ALL keywords, typeKeywords, operators, and tokenizer rules from the design doc
   - Block comments: /- ... -/ syntax

3. Create src/lib/editor/editorTheme.ts as shown in Section 4.4:
   - Export getEditorTheme() returning Monaco.editor.IStandaloneThemeData
   - Use the exact color values from the design doc (editor background #111113, cursor #7c6af7, etc.)

4. Create src/lib/editor/unicodeMapper.ts as shown in Section 4.5:
   - Export installUnicodeMapper(editor)
   - On Space/Tab keypress, check for \keyword pattern before cursor
   - Replace with Unicode character from LEAN_UNICODE_MAP (all entries from Section 4.5)

5. In MonacoEditor.svelte, call installUnicodeMapper(editor) after creation.

6. Verify: npm run check passes. Manually test by navigating to /game and confirming
   the editor loads with Lean syntax highlighting.
```

---

## Step 7: Build the Problem Panel with KaTeX Rendering

**Prompt:**

```
Implement the Problem Panel component from frontend_design.md Section 5 (ProblemPanel.svelte).

Requirements:

1. Create src/lib/components/game/ProblemPanel.svelte as shown in Section 5.

2. Props:
   - goal: string (the Lean goal, e.g., "∀ n : ℕ, n + 0 = n")
   - difficulty: number (1-5, default 1)
   - category: string (default '')
   - hint: string (default '')
   - showHint: boolean (default false, toggled by user)

3. Implement leanToKaTeX function exactly as shown in Section 5:
   - Replace ∀ → \forall, ∃ → \exists, → → \to, ↔ → \leftrightarrow
   - ∧ → \land, ∨ → \lor, ¬ → \neg
   - ℕ → \mathbb{N}, ℤ → \mathbb{Z}, ℝ → \mathbb{R}
   - ≤ → \leq, ≥ → \geq, ≠ → \neq, ⊢ → \vdash

4. Use KaTeX to render: katex.renderToString(leanToKaTeX(goal), { throwOnError: false, displayMode: true })

5. Display the raw Lean syntax below the rendered goal (for players who prefer it).

6. Difficulty stars: render 5 star icons, filled based on difficulty value.

7. Hint toggle button: clicking shows/hides hint text with a slide transition.

8. Style with the design tokens: .panel background, .status-label for headers, --font-mono for goal.

9. Verify: npm run check passes.
```

---

## Step 8: Build the Status Panel and Timer

**Prompt:**

```
Implement the Status Panel and Timer components from frontend_design.md Sections 5 and 10.

Requirements:

1. Create src/lib/components/game/StatusPanel.svelte:
   - Shows player status: "YOU" section and "OPPONENT" section
   - Displays each player's activity: idle / typing... / checking... / submitted
   - Shows submission count for each player
   - Styled as a right sidebar panel (220px wide per the game grid layout)

2. Create src/lib/ui/Timer.svelte as shown in Section 10:
   - Props: seconds: number
   - Computed: minutes = Math.floor(seconds / 60), secs = seconds % 60
   - CSS classes: .urgent (seconds <= 60, uses --warning color), .critical (seconds <= 10,
     uses --error color, pulses with animation)
   - Display format: ⏱ MM:SS
   - Font: font-variant-numeric tabular-nums for stable width

3. Create src/lib/ui/Spinner.svelte:
   - A simple CSS-only spinning animation (no external deps)
   - Props: size: number (default 16)

4. Create src/lib/ui/Badge.svelte:
   - For displaying difficulty stars or category labels
   - Props: variant: 'difficulty' | 'category', value: string | number

5. Verify: npm run check passes.
```

---

## Step 9: Build the Result Modal

**Prompt:**

```
Implement the Result Modal from frontend_design.md Section 3.4.

Requirements:

1. Create src/lib/components/game/ResultModal.svelte.

2. Props:
   - result: { won: boolean, winnerProof: string, canonicalProof: string, eloDelta: number }
   - timeElapsed: number (seconds)

3. When won === true:
   - Display: "YOU WIN!" with trophy icon
   - Show time taken (format as Xm Ys)
   - Show ELO change: "+24 rating points" (or negative if lost)
   - Show winning proof in a code block
   - Show canonical solution in a separate code block
   - Trigger confetti on mount using canvas-confetti:
     confetti({ particleCount: 150, spread: 80, origin: { y: 0.6 },
       colors: ['#7c6af7', '#34d399', '#fbbf24'] })

4. When won === false:
   - Display: "YOU LOST" with appropriate styling
   - Same info layout but muted colors

5. Two buttons at bottom: "Rematch" and "New Opponent"
   - Dispatch events: on:rematch and on:newOpponent

6. Style as a modal overlay (dark backdrop, centered card).

7. Verify: npm run check passes.
```

---

## Step 10: Assemble the Game Screen

**Prompt:**

```
Assemble the full game screen by wiring all components together, as shown in
frontend_design.md Section 3.3 and Section 5 (GameScreen.svelte).

Requirements:

1. Update src/routes/game/+page.svelte to be the full game screen:

2. Layout (from Section 3.3 wireframe):
   - Header bar: ProofBattle logo (left), player names + ELO (center), timer (right)
   - Three-column grid below header:
     - Left column (280px): ProblemPanel
     - Center column (flex-1): Monaco Editor + Submit button
     - Right column (220px): StatusPanel
   - Mobile responsive (@media max-width 900px): stack to single column

3. Wire up the WebSocket:
   - onMount: call wsClient.connect()
   - onDestroy: call wsClient.disconnect()

4. Wire up the editor:
   - Bind to MonacoEditor component ref
   - On submit button click: get code from editor, call wsClient.submitProof(code)
   - Disable submit when canSubmit is false or submitting is true

5. Show the ResultModal when gameStore.result is non-null.

6. Connect store to UI:
   - Pass gameStore.challenge to ProblemPanel
   - Pass gameStore.lspDiagnostics to MonacoEditor
   - Pass gameStore.opponentActivity to StatusPanel
   - Pass gameStore.result to ResultModal

7. Add the SubmitButton with states from Section 10:
   - Default: "Submit Proof →"
   - Submitting: Spinner + "Verifying with Lean..."
   - Success: "Proof accepted!" (green)
   - Error: "Try again" (red)

8. Verify: npm run check passes. Manual test: start the Rust server, open two browser tabs,
   and confirm the full game flow works end-to-end.
```

---

## Step 11: Build the Landing Page

**Prompt:**

```
Implement the Landing/Home screen from frontend_design.md Section 3.1.

Requirements:

1. Update src/routes/+page.svelte with the landing page layout from Section 3.1:

2. Layout:
   - Centered vertically and horizontally
   - Title: "⊢ ProofBattle" (large, --font-mono, --text-primary)
   - Subtitle: "Competitive Lean Theorem Proving"
   - Username input field (styled with .panel background)
   - Two buttons side by side:
     - "Play Now" (btn-primary, navigates to /lobby)
     - "Practice" (secondary style, no navigation yet — placeholder)

3. Bottom stats bar:
   - "Live Games: 12    Players Online: 47    Top ELO: 2140"
   - These are static for now (hardcoded values). Will be dynamic later.

4. Store the username in a writable store at src/lib/stores/userStore.ts:
   - export const userStore = writable<{ username: string | null }>({ username: null })
   - On "Play Now" click: set username, navigate to /lobby

5. Verify: npm run check passes. Landing page renders correctly at /.
```

---

## Step 12: Build the Lobby / Matchmaking Screen

**Prompt:**

```
Implement the Matchmaking/Lobby screen from frontend_design.md Section 3.2.

Requirements:

1. Update src/routes/lobby/+page.svelte with the matchmaking layout from Section 3.2:

2. Layout:
   - Centered vertically and horizontally
   - "Finding your opponent..." text
   - Animated pulse dots (three dots that fade in/out in sequence)
   - "Your ELO: 1024 — searching ±150 range" (static for now)
   - Cancel button that navigates back to /

3. On mount:
   - Call wsClient.joinQueue() to enter the matchmaking queue
   - Listen to gameStore.phase transitions:
     - If phase becomes 'matchmaking': show the searching animation
     - If phase becomes 'in_game': navigate to /game

4. Create the pulse animation using CSS keyframes:
   @keyframes pulse {
     0%, 100% { opacity: 0.3; }
     50% { opacity: 1; }
   }
   Apply to each dot with staggered animation-delay.

5. Cancel button: call wsClient.disconnect(), navigate to /.

6. Verify: npm run check passes. Manual test: open two tabs, both navigate to /lobby,
   confirm they match and redirect to /game.
```

---

## Step 13: Build the Diagnostics Bar

**Prompt:**

```
Implement the Diagnostics Bar for showing Lean error messages in real-time,
from frontend_design.md Section 9.

Requirements:

1. Create src/lib/components/game/DiagnosticsBar.svelte:
   - Positioned below the editor in the game screen (full width of center column)
   - Shows the latest Lean diagnostic from gameStore.lspDiagnostics
   - Display format: "⚠ Line {line}: {message}"
   - Color: --error for errors, --warning for warnings, --info for info
   - Collapsible: click to expand/collapse full diagnostics list
   - Styled with --bg-elevated background, --bg-border top border

2. In the game screen (src/routes/game/+page.svelte):
   - Add DiagnosticsBar below the Monaco Editor, above the Submit button
   - Pass gameStore.lspDiagnostics to it

3. Debounced diagnostics (Section 9, Approach A):
   - Create src/lib/utils/debounce.ts:
     export function debounce<T extends (...args: any[]) => any>(fn: T, ms: number): T
   - In the game screen, when the editor content changes:
     - Debounce at 800ms
     - On debounce fire: send a CheckProof message (if the backend supports it)
       or simply update the diagnostics store
   - For now, this is a placeholder — the actual LSP streaming will be added when the
     backend implements the CheckProof endpoint

4. Verify: npm run check passes.
```

---

## Step 14: Add WebSocket Reconnection on the Client

**Prompt:**

```
Implement client-side reconnection logic to work with the server's session token system
(from Step 10 of the Rust backend prompts and frontend_design.md Section 6).

Requirements:

1. In src/lib/ws/client.ts:

2. When receiving ServerMessage::SessionToken { token }:
   - Store token in localStorage: localStorage.setItem('session_token', token)

3. On connect, check for existing session token:
   - const token = localStorage.getItem('session_token')
   - If token exists: append ?token=${token} to the WebSocket URL

4. Reconnection logic (already partially in Step 4, enhance it):
   - On unexpected disconnect (onclose with wasClean === false):
     - Attempt reconnection with exponential backoff (1s, 2s, 4s, 8s, 16s, 30s max)
     - Max 5 attempts
     - On each attempt: include the session token in the URL
   - On successful reconnect with a valid session token:
     - Server will restore the player to their room
     - Client should re-read gameStore state from the server messages
   - On reconnect failure (all attempts exhausted):
     - Clear session token from localStorage
     - Navigate to / (landing page)

5. In the state machine (src/lib/ws/stateMachine.ts):
   - Add transition: game_ended → CONNECT → connecting (for rematch)
   - Add transition: error → CONNECT → connecting

6. Verify: npm run check passes.
```

---

## Step 15: Write Playwright E2E Tests

**Prompt:**

```
Write Playwright end-to-end tests for the core user flows, as specified in
frontend_design.md Section 1 (Testing: Playwright e2e).

Requirements:

1. The Rust backend must be running on ws://localhost:3000/ws for these tests.
   Use the project's existing Rust server (battle/) started with: cargo run
   (from the battle/ directory).

2. Create playwright.config.ts at the frontend project root:
   - baseURL: http://localhost:5173
   - webServer: { command: 'npm run dev', port: 5173, reuseExistingServer: true }
   - Use Chromium browser

3. Create tests/e2e/landing.spec.ts:
   - test('landing page renders title and buttons'):
     Navigate to /
     Assert: page contains "ProofBattle"
     Assert: "Play Now" button is visible
     Assert: "Practice" button is visible

   - test('entering username and clicking Play Now navigates to lobby'):
     Type "testplayer" in the username input
     Click "Play Now"
     Assert: URL is /lobby

4. Create tests/e2e/game-flow.spec.ts:
   This test requires TWO browser contexts (two players).

   - test('two players can match and see the same challenge'):
     1. Open two browser contexts (page1, page2)
     2. Both navigate to /, enter usernames, click Play Now
     3. Both navigate to /lobby
     4. Wait for both to be redirected to /game (matchmaking completes)
     5. On both pages, assert the ProblemPanel shows a goal
     6. Assert both pages show the SAME goal text

   - test('first player to submit valid proof wins'):
     1. Two players matched in /game
     2. Player 1 types "exact Nat.add_zero n" in the editor (or whatever proof works)
     3. Player 1 clicks Submit
     4. Wait for ResultModal to appear on Player 1's page
     5. Assert: ResultModal shows "YOU WIN!"
     6. Assert: Player 2's page shows "YOU LOST"

5. Run tests: npx playwright test
   Note: tests may need adjustment based on actual challenge goals and proof syntax.
```

---

> **After completing all 15 steps**, the frontend will have:
> - SvelteKit project with Tailwind CSS v4 and design token system
> - Three routed pages: Landing, Lobby, Game
> - Monaco Editor with Lean 4 syntax highlighting, Monarch grammar, and Unicode input
> - KaTeX-rendered proof goals in the Problem Panel
> - WebSocket client with state machine, reconnection, and session tokens
> - Svelte store managing all game state (phase, challenge, diagnostics, results)
> - Timer with visual urgency states
> - Result modal with confetti on win
> - Diagnostics bar for Lean error feedback
> - Playwright E2E tests covering the full two-player flow
> - All styled with the dark math terminal aesthetic from the design doc
