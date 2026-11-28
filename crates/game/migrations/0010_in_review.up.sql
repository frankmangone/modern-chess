-- A game waiting for staff to look at it (decision 9 of the ownership plan: the first game of a
-- new author is reviewed). The state machine in `crates/game/src/domain/lifecycle.rs` gains
-- IN_REVIEW and four moves; the check and the trigger follow it, so a bug cannot write a move the
-- machine does not have.
ALTER TABLE user_games DROP CONSTRAINT user_games_status_check;
ALTER TABLE user_games ADD CONSTRAINT user_games_status_check
    CHECK (status IN ('DRAFT', 'IN_REVIEW', 'PUBLISHED', 'UNPUBLISHED', 'REMOVED'));

CREATE OR REPLACE FUNCTION user_games_check_status_move() RETURNS trigger AS $$
BEGIN
    IF NEW.status = OLD.status THEN
        RETURN NEW;
    END IF;
    IF (OLD.status, NEW.status) IN (
        ('DRAFT', 'IN_REVIEW'),
        ('IN_REVIEW', 'DRAFT'),
        ('IN_REVIEW', 'PUBLISHED'),
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
