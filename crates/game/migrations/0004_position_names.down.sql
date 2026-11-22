-- Turn grid square names back into x and y; any other name has no coordinates and stays as it is.
UPDATE game_commands
SET kind = (kind - 'position')
    || jsonb_build_object(
        'x', split_part(kind->>'position', ',', 1)::numeric,
        'y', split_part(kind->>'position', ',', 2)::numeric
    )
WHERE kind->>'position' ~ '^[0-9]+,[0-9]+$';
