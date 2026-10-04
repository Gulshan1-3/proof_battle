export interface MatchSummary {
	id: string;
	room_id: string;
	opponent_id: string;
	opponent_username: string | null;
	opponent_elo: number;
	outcome: string;
	elo_delta: number;
	problem_goal: string;
	problem_category: string;
	problem_difficulty: number;
	rated: boolean;
	duration_ms: number;
	created_at: string;
}

export interface MatchDetail {
	id: string;
	room_id: string;
	player1_id: string;
	player1_username: string | null;
	player1_elo_before: number;
	player1_elo_after: number;
	elo_delta_p1: number;
	player2_id: string;
	player2_username: string | null;
	player2_elo_before: number;
	player2_elo_after: number;
	elo_delta_p2: number;
	problem_id: string | null;
	problem_goal: string;
	problem_category: string;
	problem_difficulty: number;
	winner_id: string | null;
	outcome: string;
	winning_proof: string | null;
	canonical_proof: string | null;
	rated: boolean;
	duration_ms: number;
	created_at: string;
}

export interface CategoryStats {
	category: string;
	games_played: number;
	wins: number;
	win_rate: number;
}

export interface PlayerStats {
	player_id: string;
	games_played: number;
	wins: number;
	losses: number;
	draws: number;
	win_rate: number;
	current_rating: number;
	categories: CategoryStats[];
}

export async function fetchPlayerHistory(playerId: string, limit = 20): Promise<MatchSummary[]> {
	const res = await fetch(`/api/history?player_id=${encodeURIComponent(playerId)}&limit=${limit}`);
	if (!res.ok) {
		throw new Error(`Failed to fetch history: ${res.statusText}`);
	}
	return res.json();
}

export async function fetchMatchDetail(matchId: string): Promise<MatchDetail | null> {
	const res = await fetch(`/api/matches/${encodeURIComponent(matchId)}`);
	if (res.status === 404) {
		return null;
	}
	if (!res.ok) {
		throw new Error(`Failed to fetch match detail: ${res.statusText}`);
	}
	return res.json();
}

export async function fetchPlayerStats(playerId: string): Promise<PlayerStats> {
	const res = await fetch(`/api/stats?player_id=${encodeURIComponent(playerId)}`);
	if (!res.ok) {
		throw new Error(`Failed to fetch player stats: ${res.statusText}`);
	}
	return res.json();
}
