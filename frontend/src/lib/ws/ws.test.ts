import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
	type ConnState,
	type ConnEvent,
	TRANSITIONS,
	getNextState,
	IllegalStateTransitionError
} from './state-machine';
import { ProofBattleClient } from './client.svelte';
import { game } from '$lib/stores/game.svelte';
import type { ClientMessage, ServerMessage } from '$lib/ws/generated';

describe('Connection State Machine & Transitions', () => {
	it('walks every legal transition declared in TRANSITIONS table', () => {
		for (const [fromState, events] of Object.entries(TRANSITIONS) as [
			ConnState,
			Record<ConnEvent, ConnState>
		][]) {
			for (const [event, expectedNext] of Object.entries(events) as [ConnEvent, ConnState][]) {
				const next = getNextState(fromState, event, true);
				expect(next).toBe(expectedNext);
			}
		}
	});

	it('throws IllegalStateTransitionError on every illegal transition in dev mode', () => {
		const allStates: ConnState[] = [
			'disconnected',
			'connecting',
			'connected_idle',
			'matchmaking',
			'game_starting',
			'in_game',
			'submitting',
			'reconnecting',
			'reattaching',
			'game_ended',
			'error'
		];

		const allEvents: ConnEvent[] = [
			'CONNECT',
			'OPEN',
			'JOIN_QUEUE',
			'LEAVE_QUEUE',
			'MSG_WELCOME',
			'MSG_MATCH_FOUND',
			'MSG_ROUND_START',
			'SUBMIT_PROOF',
			'MSG_VERDICT_REJECTED',
			'MSG_ROUND_END',
			'RESIGN',
			'DISCONNECT',
			'RECONNECT_ATTEMPT',
			'REATTACH_SUCCESS',
			'ERROR',
			'RESET'
		];

		let testedIllegalTransitions = 0;

		for (const state of allStates) {
			for (const event of allEvents) {
				const isLegal = TRANSITIONS[state]?.[event] !== undefined;
				if (!isLegal) {
					testedIllegalTransitions++;
					expect(() => getNextState(state, event, true)).toThrowError(IllegalStateTransitionError);
				}
			}
		}

		expect(testedIllegalTransitions).toBeGreaterThan(50);
	});
});

interface MockClient {
	state: ConnState;
	handleMessage: (msg: ServerMessage) => void;
	send: (msg: ClientMessage) => void;
	submitProof: (code: string) => Promise<{ reqId: string; verdict: string; checkSkipped: boolean }>;
	checkProof: (code: string) => Promise<{ reqId: string; verdict: string; checkSkipped: boolean }>;
}

describe('ProofBattleClient & Request Resolution', () => {
	let client: ProofBattleClient;
	let mockClient: MockClient;

	beforeEach(() => {
		vi.useFakeTimers();
		client = new ProofBattleClient();
		mockClient = client as unknown as MockClient;
		game.reset();
	});

	afterEach(() => {
		vi.useRealTimers();
	});

	it('synchronizes clock offset from server time', () => {
		mockClient.state = 'connecting';
		const localNow = 1_000_000;
		vi.setSystemTime(localNow);

		const serverTimeMs = 1_005_250; // Server is 5250ms ahead
		mockClient.handleMessage({
			type: 'Welcome',
			player_id: 'p1',
			session_token: 'tok-xyz',
			server_time_ms: BigInt(serverTimeMs),
			heartbeat_interval_ms: 15000n,
			protocol_version: 1
		});

		expect(game.serverTimeOffsetMs).toBe(5250);
		expect(game.playerId).toBe('p1');
		expect(game.sessionToken).toBe('tok-xyz');
	});

	it('correlates req_id for submitProof and resolves on Verdict', async () => {
		mockClient.state = 'in_game';
		let sentPayload: ClientMessage | null = null;
		mockClient.send = vi.fn((msg: ClientMessage) => {
			sentPayload = msg;
		});

		const promise = mockClient.submitProof('intro n; simp');
		if (!sentPayload || (sentPayload as ClientMessage).type !== 'ProofRequest') {
			throw new Error('Expected ProofRequest');
		}
		const request = sentPayload as Extract<ClientMessage, { type: 'ProofRequest' }>;
		expect(request.type).toBe('ProofRequest');
		expect(request.intent).toBe('Submit');
		const reqId = request.req_id;

		mockClient.handleMessage({
			type: 'Verdict',
			req_id: reqId,
			verdict: 'Accepted',
			reason: null,
			message: 'Proof accepted',
			diagnostics: [],
			elapsed_ms: 120n,
			check_skipped: false
		});

		const result = await promise;
		expect(result.verdict).toBe('Accepted');
		expect(result.reqId).toBe(reqId);
		expect(game.isSubmitting).toBe(false);
	});

	it('resolves checkProof with check_skipped: true without hanging', async () => {
		mockClient.state = 'in_game';
		let sentPayload: ClientMessage | null = null;
		mockClient.send = vi.fn((msg: ClientMessage) => {
			sentPayload = msg;
		});

		const promise = mockClient.checkProof('simp');
		if (!sentPayload || (sentPayload as ClientMessage).type !== 'ProofRequest') {
			throw new Error('Expected ProofRequest');
		}
		const request = sentPayload as Extract<ClientMessage, { type: 'ProofRequest' }>;
		const reqId = request.req_id;

		mockClient.handleMessage({
			type: 'Verdict',
			req_id: reqId,
			verdict: 'Rejected',
			reason: { type: 'Busy' },
			message: 'Check lane saturated',
			diagnostics: [],
			elapsed_ms: 5n,
			check_skipped: true
		});

		const result = await promise;
		expect(result.checkSkipped).toBe(true);
		expect(game.checkSkipped).toBe(true);
		expect(game.isChecking).toBe(false);
	});
});
