-- Commands logged before choose and cancel become the answers they were. A drop was one atomic
-- command carrying both answers: the log type reads it, the API never accepts it.
UPDATE game_commands
SET kind = jsonb_build_object('type', 'choose', 'place', kind->'position')
WHERE kind->>'type' IN ('select', 'move');

UPDATE game_commands
SET kind = jsonb_build_object('type', 'choose', 'code', kind->'code')
WHERE kind->>'type' = 'promote';

UPDATE game_commands
SET kind = jsonb_build_object('type', 'choose', 'action', 'PASS')
WHERE kind->>'type' = 'pass';

UPDATE game_commands
SET kind = jsonb_build_object('type', 'choose', 'hand', kind->'code', 'place', kind->'position')
WHERE kind->>'type' = 'drop';
