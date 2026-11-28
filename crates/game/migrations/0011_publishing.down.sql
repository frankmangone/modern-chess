DROP TRIGGER user_game_revisions_fixed ON user_game_revisions;
DROP FUNCTION user_game_revisions_never_change();
DROP TABLE user_game_revisions;
DROP TRIGGER user_games_slug_kept ON user_games;
DROP FUNCTION user_games_keep_slug();
ALTER TABLE user_games
    DROP COLUMN author_handle,
    DROP COLUMN licence,
    DROP COLUMN description,
    DROP COLUMN tagline,
    DROP COLUMN slug;
