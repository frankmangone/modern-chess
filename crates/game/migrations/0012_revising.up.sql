-- Changing a published game (decision 3: a revision never changes, an edit makes a new one).
--
-- The author's "Edit" makes a new draft, a working copy, that remembers the game it revises. Publishing
-- the working copy adds the next revision to the original game, under the same address, and the
-- working copy is deleted. A game has at most one working copy at a time.
ALTER TABLE user_games
    ADD COLUMN revises_game_id UUID REFERENCES user_games (id) ON DELETE RESTRICT
        CHECK (revises_game_id IS NULL OR revises_game_id <> id);

CREATE UNIQUE INDEX uq_user_games_one_working_copy
    ON user_games (revises_game_id) WHERE revises_game_id IS NOT NULL;
