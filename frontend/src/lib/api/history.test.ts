import { describe, it, expect, vi, beforeEach } from 'vitest';
import { fetchPlayerHistory, fetchMatchDetail, fetchPlayerStats } from './history';

describe('history api client', () => {
	beforeEach(() => {
		vi.restoreAllMocks();
	});

	it('fetchPlayerHistory returns list of matches', async () => {
		const mockMatches = [
			{
				id: 'm1',
				room_id: 'r1',
				opponent_id: 'p2',
				opponent_username: 'Player2',
				opponent_elo: 1200,
				outcome: 'Won',
				elo_delta: 16,
				problem_goal: 'n + 0 = n',
				problem_category: 'logic',
				problem_difficulty: 1,
				rated: true,
				duration_ms: 1200,
				created_at: '2026-10-05T00:00:00Z'
			}
		];

		const fetchMock = vi.fn().mockResolvedValue({
			ok: true,
			json: async () => mockMatches
		});
		vi.stubGlobal('fetch', fetchMock);

		const res = await fetchPlayerHistory('p1', 10);
		expect(res).toEqual(mockMatches);
		expect(fetchMock).toHaveBeenCalledWith('/api/history?player_id=p1&limit=10');
	});

	it('fetchMatchDetail returns null on 404', async () => {
		const fetchMock = vi.fn().mockResolvedValue({
			status: 404,
			ok: false
		});
		vi.stubGlobal('fetch', fetchMock);

		const res = await fetchMatchDetail('non-existent');
		expect(res).toBeNull();
		expect(fetchMock).toHaveBeenCalledWith('/api/matches/non-existent');
	});

	it('fetchMatchDetail returns detail object on 200', async () => {
		const mockDetail = {
			id: 'm1',
			room_id: 'r1',
			player1_id: 'p1',
			player1_username: 'Alice',
			player1_elo_before: 1200,
			player1_elo_after: 1216,
			elo_delta_p1: 16,
			player2_id: 'p2',
			player2_username: 'Bob',
			player2_elo_before: 1200,
			player2_elo_after: 1184,
			elo_delta_p2: -16,
			problem_id: null,
			problem_goal: 'n + 0 = n',
			problem_category: 'arithmetic',
			problem_difficulty: 1,
			winner_id: 'p1',
			outcome: 'Won',
			winning_proof: 'intro n\nsimp',
			canonical_proof: null,
			rated: true,
			duration_ms: 2500,
			created_at: '2026-10-05T00:00:00Z'
		};

		const fetchMock = vi.fn().mockResolvedValue({
			status: 200,
			ok: true,
			json: async () => mockDetail
		});
		vi.stubGlobal('fetch', fetchMock);

		const res = await fetchMatchDetail('m1');
		expect(res).toEqual(mockDetail);
	});

	it('fetchPlayerStats returns stats summary', async () => {
		const mockStats = {
			player_id: 'p1',
			games_played: 10,
			wins: 7,
			losses: 3,
			draws: 0,
			win_rate: 70.0,
			current_rating: 1350,
			categories: [
				{
					category: 'logic',
					games_played: 5,
					wins: 4,
					win_rate: 80.0
				}
			]
		};

		const fetchMock = vi.fn().mockResolvedValue({
			ok: true,
			json: async () => mockStats
		});
		vi.stubGlobal('fetch', fetchMock);

		const res = await fetchPlayerStats('p1');
		expect(res).toEqual(mockStats);
		expect(fetchMock).toHaveBeenCalledWith('/api/stats?player_id=p1');
	});
});
