# Reference

This page is the complete reference for the spec format. It is the file `specs/SPEC_FORMAT.md`
in the repository, shown here unchanged, so it is always the same text the engine team edits.
It is long, and meant to be searched (the search box is at the top of the page) rather than read
from the top.

**Find your way**

- The file as a whole: [top-level structure](#top-level-structure), then the parts in order:
  [board](#board), [players](#players), [end checks](#end_checks), [turns](#turns),
  [zones](#zones), [pieces](#pieces), [moves](#move-definitions), [actions](#actions).
- Tests, regions and filters: [Tests](#tests). Rules on every move:
  [`every_action`, `must`](#rules-that-apply-to-every-move) and [`repeat`](#repeat).
- Bags, hands and places that hold many: [bags and the hand](#bags-and-the-hand),
  [places that hold many, and Kalah](#places-that-hold-many-and-kalah).
- Chance and secrets: [dice and shuffles](#dice-and-shuffles),
  [hidden information](#hidden-information),
  [sides with their own rules](#sides-with-their-own-rules).
- Go-like games: [placing a piece](#placing-a-piece), [passing](#passing),
  [liberty capture](#liberty-capture), [repeated positions](#repeated-positions).
- Looking for the old name of something? [Retired forms](#retired-forms).
- New to this? Start with the [tutorial](introduction.md) instead.

---

{{#include ../../../specs/SPEC_FORMAT.md}}
