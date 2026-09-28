CREATE EXTENSION IF NOT EXISTS "pgcrypto";

CREATE TABLE IF NOT EXISTS problems (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug TEXT UNIQUE NOT NULL,
    goal TEXT NOT NULL,
    imports TEXT[] NOT NULL,
    statement TEXT NOT NULL,
    canonical_proof TEXT NOT NULL,
    difficulty SMALLINT NOT NULL CHECK (difficulty BETWEEN 1 AND 10),
    category TEXT NOT NULL,
    tactic_hint TEXT,
    source_theorem TEXT,
    source_url TEXT,
    verified BOOLEAN NOT NULL DEFAULT FALSE,
    verified_at TIMESTAMPTZ,
    verification_error TEXT,
    times_played INT NOT NULL DEFAULT 0,
    last_played_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_problems_difficulty ON problems (difficulty) WHERE verified;
CREATE INDEX IF NOT EXISTS idx_problems_category_difficulty ON problems (category, difficulty) WHERE verified;
