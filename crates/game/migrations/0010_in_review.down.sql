-- Anything in review goes back to being a draft first, so the old check holds.
ALTER TABLE user_games DISABLE TRIGGER user_games_status_move;
UPDATE user_games SET status = 'DRAFT' WHERE status = 'IN_REVIEW';
ALTER TABLE user_games ENABLE TRIGGER user_games_status_move;

ALTER TABLE user_games DROP CONSTRAINT user_games_status_check;
ALTER TABLE user_games ADD CONSTRAINT user_games_status_check
    CHECK (status IN ('DRAFT', 'PUBLISHED', 'UNPUBLISHED', 'REMOVED'));

CREATE OR REPLACE FUNCTION user_games_check_status_move() RETURNS trigger AS $$
BEGIN
    IF NEW.status = OLD.status THEN
        RETURN NEW;
    END IF;
    IF (OLD.status, NEW.status) IN (
        ('DRAFT', 'PUBLISHED'),
        ('PUBLISHED', 'UNPUBLISHED'),
        ('UNPUBLISHED', 'PUBLISHED'),
        ('PUBLISHED', 'REMOVED'),
        ('UNPUBLISHED', 'REMOVED'),
        ('REMOVED', 'UNPUBLISHED')
    ) THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'a game cannot move from % to %', OLD.status, NEW.status
        USING ERRCODE = 'check_violation';
END;
$$ LANGUAGE plpgsql;
