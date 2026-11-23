-- Takes the key out again. A version-1 spec is the only kind there is, so nothing else is lost.
UPDATE game_specs
SET rules = rules - 'format_version';
