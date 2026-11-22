-- Turns each choose back into the command it came from. A select and a move were both logged
-- as a place, so every place comes back as a select. A cancel, a hand piece on its own and an
-- action other than PASS have no earlier form and are left as they are.
UPDATE game_commands
SET kind = jsonb_build_object('type', 'drop', 'code', kind->'hand', 'position', kind->'place')
WHERE kind->>'type' = 'choose' AND kind ? 'hand' AND kind ? 'place';

UPDATE game_commands
SET kind = jsonb_build_object('type', 'select', 'position', kind->'place')
WHERE kind->>'type' = 'choose'
  AND kind ? 'place' AND NOT kind ? 'hand' AND NOT kind ? 'code' AND NOT kind ? 'action';

UPDATE game_commands
SET kind = jsonb_build_object('type', 'promote', 'code', kind->'code')
WHERE kind->>'type' = 'choose'
  AND kind ? 'code' AND NOT kind ? 'place' AND NOT kind ? 'hand' AND NOT kind ? 'action';

UPDATE game_commands
SET kind = jsonb_build_object('type', 'pass')
WHERE kind->>'type' = 'choose' AND kind->>'action' = 'PASS'
  AND NOT kind ? 'place' AND NOT kind ? 'hand' AND NOT kind ? 'code';
