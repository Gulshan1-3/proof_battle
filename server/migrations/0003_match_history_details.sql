-- Migration 0003: Match History Details and Replay Support

ALTER TABLE match_history ADD COLUMN IF NOT EXISTS winning_proof TEXT;
ALTER TABLE match_history ADD COLUMN IF NOT EXISTS canonical_proof TEXT;
ALTER TABLE match_history ADD COLUMN IF NOT EXISTS player1_username TEXT;
ALTER TABLE match_history ADD COLUMN IF NOT EXISTS player2_username TEXT;
ALTER TABLE match_history ADD COLUMN IF NOT EXISTS problem_goal TEXT NOT NULL DEFAULT '';
ALTER TABLE match_history ADD COLUMN IF NOT EXISTS problem_category TEXT NOT NULL DEFAULT '';
ALTER TABLE match_history ADD COLUMN IF NOT EXISTS problem_difficulty SMALLINT NOT NULL DEFAULT 1;
ALTER TABLE match_history ADD COLUMN IF NOT EXISTS elo_delta_p1 INT NOT NULL DEFAULT 0;
ALTER TABLE match_history ADD COLUMN IF NOT EXISTS elo_delta_p2 INT NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_match_history_problem_category ON match_history (problem_category);
