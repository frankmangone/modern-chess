-- Rules now state the spec format they are written for. Every game stored so far was written for
-- version 1, so each stored version of the rules is given that key. The rules are otherwise
-- unchanged: matches in progress keep the rules they started with. `rules_hash` is left as it was
-- (the hash of the rules before the key), so the next seeding stores the same rules once more as
-- a new version, with the hash of what the file says.
UPDATE game_specs
SET rules = rules || '{"format_version": 1}'::jsonb
WHERE NOT rules ? 'format_version';
