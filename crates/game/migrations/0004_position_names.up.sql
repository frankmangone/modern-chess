-- Commands logged before positions were named carry x and y; give them the grid square's name.
UPDATE game_commands
SET kind = (kind - 'x' - 'y') || jsonb_build_object('position', (kind->>'x') || ',' || (kind->>'y'))
WHERE kind ? 'x' AND kind ? 'y';
