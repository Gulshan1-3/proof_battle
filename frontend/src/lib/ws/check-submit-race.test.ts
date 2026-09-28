import { describe, it, expect, beforeEach, vi } from 'vitest';
import { GameState } from '$lib/stores/game.svelte';
import { ProofBattleClient } from './client.svelte';
import type { ServerMessage, Diagnostic } from './generated';

describe('Check vs Submit Race Conditions (S8-P2)', () => {
	let client: ProofBattleClient;
	let state: GameState;
	let mockSend: ReturnType<typeof vi.fn>;

	beforeEach(() => {
		mockSend = vi.fn();
		client = new ProofBattleClient();
		// Inject mock WebSocket transport
		const mockWs = {
			readyState: 1, // OPEN
			send: mockSend,
			close: vi.fn()
		};
		// @ts-expect-error accessing private property for test injection
		client.ws = mockWs;
		client.state = 'in_game';

		state = new GameState();
		state.roomId = 'test-room';
	});

	it('ensures a late-arriving check verdict does NOT overwrite an active submit verdict', async () => {
		const submitDiag: Diagnostic = {
			line: 2,
			col: 5,
			end_line: 2,
			end_col: 10,
			severity: 'error',
			message: 'Submit Error: type mismatch'
		};

		const checkDiag: Diagnostic = {
			line: 1,
			col: 1,
			end_line: 1,
			end_col: 4,
			severity: 'warning',
			message: 'Check Warning: unused variable'
		};

		// 1. Player triggers Check (req_id: req-check-1)
		const checkPromise = client.checkProof('intro n\nexact foo');
		// @ts-expect-error reading pending requests
		const checkReqId = Array.from(client.pendingRequests.keys())[0]!;

		// 2. While Check is in flight, player presses Submit (req_id: req-submit-2)
		const submitPromise = client.submitProof('intro n\nexact foo');
		// @ts-expect-error reading pending requests
		const submitReqId = Array.from(client.pendingRequests.keys())[1]!;

		// 3. Submit verdict returns FIRST (e.g. server processes submit lane with priority)
		const submitMsg: ServerMessage = {
			type: 'Verdict',
			req_id: submitReqId,
			verdict: 'Rejected',
			reason: { type: 'LeaningFailed' },
			message: 'Official match submission failed',
			diagnostics: [submitDiag],
			elapsed_ms: 200n,
			check_skipped: false
		};
		// @ts-expect-error dispatching message
		client.handleMessage(submitMsg);
		await submitPromise;

		// Verify submit diagnostics are recorded
		const { game } = await import('$lib/stores/game.svelte');
		expect(game.submitDiagnostics).toEqual([submitDiag]);
		expect(game.lastSubmitVerdict?.reqId).toBe(submitReqId);
		expect(game.activeDiagnostics).toEqual([submitDiag]);

		// 4. Delayed Check verdict arrives LATER
		const checkMsg: ServerMessage = {
			type: 'Verdict',
			req_id: checkReqId,
			verdict: 'Rejected',
			reason: null,
			message: 'Stale check result',
			diagnostics: [checkDiag],
			elapsed_ms: 850n,
			check_skipped: false
		};
		// @ts-expect-error dispatching message
		client.handleMessage(checkMsg);
		await checkPromise;

		// ASSERTION: Submit diagnostics MUST NOT have been overwritten by delayed check!
		expect(game.submitDiagnostics).toEqual([submitDiag]);
		expect(game.lastSubmitVerdict?.reqId).toBe(submitReqId);
		expect(game.activeDiagnostics).toEqual([submitDiag]);
		expect(game.checkDiagnostics).toEqual([checkDiag]);
	});
});
