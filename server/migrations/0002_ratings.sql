CREATE TABLE IF NOT EXISTS players (
    id UUID PRIMARY KEY,
    username TEXT UNIQUE,
    elo INT NOT NULL DEFAULT 1200,
    glicko_r NUMERIC,
    glicko_rd NUMERIC,
    glicko_vol NUMERIC,
    games_played INT NOT NULL DEFAULT 0,
    wins INT NOT NULL DEFAULT 0,
    losses INT NOT NULL DEFAULT 0,
    draws INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS match_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    room_id UUID NOT NULL,
    player1_id UUID NOT NULL REFERENCES players(id),
    player2_id UUID NOT NULL REFERENCES players(id),
    problem_id UUID REFERENCES problems(id),
    winner_id UUID REFERENCES players(id),
    outcome TEXT NOT NULL,
    p1_elo_before INT NOT NULL,
    p1_elo_after INT NOT NULL,
    p2_elo_before INT NOT NULL,
    p2_elo_after INT NOT NULL,
    rated BOOLEAN NOT NULL DEFAULT TRUE,
    duration_ms BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_match_history_player1 ON match_history (player1_id);
CREATE INDEX IF NOT EXISTS idx_match_history_player2 ON match_history (player2_id);
CREATE INDEX IF NOT EXISTS idx_match_history_created_at ON match_history (created_at DESC);
