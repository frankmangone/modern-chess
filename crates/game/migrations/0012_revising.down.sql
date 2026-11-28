DROP INDEX uq_user_games_one_working_copy;
-- A working copy without its original is only a draft: it stays, and is just no longer a revision.
ALTER TABLE user_games DROP COLUMN revises_game_id;
