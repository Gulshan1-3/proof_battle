import { describe, it, expect } from 'vitest';
import { getNextState, type ConnState, type ConnEvent } from './state-machine';

describe('Exhaustive Expected Transitions Specification', () => {
	const REQUIRED_LEGAL_TRANSITIONS: [ConnState, ConnEvent, ConnState][] = [
		['disconnected', 'CONNECT', 'connecting'],
		['disconnected', 'RECONNECT_ATTEMPT', 'reconnecting'],
		['connecting', 'OPEN', 'connected_idle'],
		['connecting', 'MSG_WELCOME', 'connected_idle'],
		['connecting', 'DISCONNECT', 'disconnected'],
		['connecting', 'ERROR', 'error'],
		['connected_idle', 'JOIN_QUEUE', 'matchmaking'],
		['connected_idle', 'DISCONNECT', 'disconnected'],
		['matchmaking', 'LEAVE_QUEUE', 'connected_idle'],
		['matchmaking', 'MSG_MATCH_FOUND', 'game_starting'],
		['matchmaking', 'DISCONNECT', 'disconnected'],
		['game_starting', 'MSG_ROUND_START', 'in_game'],
		['game_starting', 'DISCONNECT', 'reconnecting'],
		['in_game', 'SUBMIT_PROOF', 'submitting'],
		['in_game', 'MSG_ROUND_END', 'game_ended'],
		['in_game', 'RESIGN', 'game_ended'],
		['in_game', 'DISCONNECT', 'reconnecting'],
		['submitting', 'MSG_VERDICT_REJECTED', 'in_game'],
		['submitting', 'MSG_ROUND_END', 'game_ended'],
		['submitting', 'DISCONNECT', 'reconnecting'],
		['reconnecting', 'CONNECT', 'connecting'],
		['reconnecting', 'REATTACH_SUCCESS', 'in_game'],
		['reconnecting', 'DISCONNECT', 'disconnected'],
		['reattaching', 'REATTACH_SUCCESS', 'in_game'],
		['reattaching', 'MSG_ROUND_START', 'in_game'],
		['game_ended', 'JOIN_QUEUE', 'matchmaking'],
		['game_ended', 'RESET', 'connected_idle'],
		['error', 'CONNECT', 'connecting'],
		['error', 'RESET', 'disconnected']
	];

	it('verifies all expected legal transitions are present and correct', () => {
		for (const [from, event, expected] of REQUIRED_LEGAL_TRANSITIONS) {
			const next = getNextState(from, event, true);
			expect(next, `Transition from ${from} via ${event} should be ${expected}`).toBe(expected);
		}
	});
});
