# Reference

This page is the complete reference for the spec format. It is the file `specs/SPEC_FORMAT.md`
in the repository, shown here unchanged, so it is always the same text the engine team edits.

Some things the tutorial covers that this reference does **not** list yet:

- the top-level flags `mandatory_capture` and `jump_enabled` (see [chapter 9](09-forced-captures.md)),
- the `player` field of the `PIECE_AT` condition, which takes `"SELF"`, `"ENEMY"` or `"ANY"`
  (see [chapter 9](09-forced-captures.md)),
- the `players` field on a move, which restricts it to the listed players (see
  [chapter 4](04-conditions-and-state.md)),
- the optional `position` of `PATH_PIECE_COUNT`, which measures the path to that relative square
  (see [chapter 4](04-conditions-and-state.md)),
- the `duration` of `SET_STATE` counting single moves by either player, not full rounds
  (see [chapter 4](04-conditions-and-state.md)).

---

{{#include ../../../specs/SPEC_FORMAT.md}}
