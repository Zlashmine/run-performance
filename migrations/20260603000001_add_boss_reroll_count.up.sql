ALTER TABLE monthly_missions
    ADD COLUMN IF NOT EXISTS boss_reroll_count INT NOT NULL DEFAULT 0;
