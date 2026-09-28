import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { MockWebSocket } from './server';
import { wsClient } from '$lib/ws/client';
import { game } from '$lib/stores/game.svelte';

describe('MockWebSocket Server Integration', () => {
	beforeEach(() => {
		MockWebSocket.install();
	});

	afterEach(() => {
		wsClient.disconnect();
		MockWebSocket.restore();
	});

	it('completes the handshake, queue, and match pairing lifecycle', async () => {
		wsClient.connect();

		// Wait 100ms for Welcome
		await new Promise((r) => setTimeout(r, 100));
		expect(wsClient.state).toBe('connected_idle');

		// Join queue
		wsClient.joinQueue();
		expect(wsClient.state).toBe('matchmaking');

		// Wait 1000ms for match pairing and round start
		await new Promise((r) => setTimeout(r, 1000));
		expect(wsClient.state).toBe('in_game');
		expect(game.roomId).toBe('mock-room-alpha');
		expect(game.problem).toBeDefined();
	});

	it('completes the practice join lifecycle with bot opponent', async () => {
		wsClient.connect();

		// Wait 100ms for Welcome
		await new Promise((r) => setTimeout(r, 100));
		expect(wsClient.state).toBe('connected_idle');

		// Join practice
		wsClient.practiceJoin();
		expect(wsClient.state).toBe('matchmaking');

		// Wait 300ms for match pairing and round start
		await new Promise((r) => setTimeout(r, 300));
		expect(wsClient.state).toBe('in_game');
		expect(game.opponent?.username).toBe('Lean Practice Bot');
		expect(game.problem).toBeDefined();
	});
});
