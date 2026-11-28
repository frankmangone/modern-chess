# Communities around games

Status: **draft for Frank's rulings** (28 Nov 2026). The shape below is what Frank decided; the open
decisions at the end are the ones that change what gets built.

## What Frank decided

- **Every game is a community.** There is no separate thing to create. A game's page is its
  community, for catalogue games and published ones alike.
- **They form organically, with no moderators at first.** The author of a game does not run its
  community. (Reports still go to platform staff, as decision 6 of the ownership plan says.)
- **It lives on the social service.**
- **People want records on games.** A win/draw/loss record, at least. An Elo-style rating is not
  decided now.
- **What goes on in a community:** discussion threads, variant proposals, puzzles and drills, and more
  later.

## Where things live today (what the design starts from)

- The social service (`crates/social`) is accounts only: users, identities, handles, guests. Its own
  database.
- The game service owns games, specs, matches and the engine. A match is a `game_instances` row with
  its `outcome` (`Won { seat }` or `Draw`, resigning is a win for the other seat) and `game_players`
  (`user_id`, no foreign key, since users live in the other database). That is enough to count
  wins, draws and losses for a person on a game, today, with no new data.
- A "game" for people is a game page with its variants under it; matches know their variant's slug.
  A community belongs to the page, so every variant shares it.

## The boundary question

The community is the social service's, but two of its parts need things only the game service has:
records need matches, and puzzles need the engine to check a solution. Two ways to place them:

- **A. The social service holds what people say; the game service serves what the games know.**
  Threads, posts, reports and (later) proposals live in social, keyed by the game page's slug the way
  `user_id` is already an opaque id across the two services. Records and puzzles are the game
  service's, since it owns matches, specs and the sandbox. The page for a game puts them side by side.
  No new link between the services.
- B. Everything lives in social, and the game service reports each finished match to it. The social
  service would then hold a copy of results and would still have to ask the game service to check a
  puzzle. More coupling, and a second copy of the matches to keep right.

**Recommendation: A.** It keeps "the community lives on the social service" true for what is
social (the talk, the people, the reports), and keeps game facts where the games are. It is also the
smaller build: records need no new tables.

## The parts

**Records** (game service). For a person on a game: wins, draws, losses, counted from finished
matches. Decisions to make are in the open list. Rating is left out; if it is wanted later it is
computed from the same finished matches in order of finish, so nothing is lost by waiting.

**Discussion** (social service). Threads and posts on a game, newest activity first. Posting needs a
signed-in account with its handle chosen; a guest can read. Reports on a post go to a queue only
staff see (decision 6), and staff can hide a post. No moderators beyond that, as decided. Replies are
flat at first (a thread is a list), since nesting is easy to add and hard to take away.

**Variant proposals** (social + game). A proposal is a thread that carries a spec, using the private
link a draft can already be shared by. It is accepted by the game's author, who publishes it as a
variant (builder slice 3b), since the ownership plan says what is listed under a game is the
author's say. Waits on 3b.

**Puzzles and drills** (game service). A saved position plus the solution, checked by the engine, with
progress for each person. Its own design when reached: who may write puzzles, and what a "solution" is
for games with dice or hidden information.

## Order

1. **Records.** Smallest, worth having at once, and it needs only the game service and the game page.
2. **Discussion**, with reports. Needs a new corner of the social service and a thread page.
3. **Puzzles and drills**, with their own design.
4. **Variant proposals**, after slice 3b.

Each is its own branch and PR. Steps 3 and 4 get a design of their own before code.

## Open decisions (rulings wanted)

1. **The boundary:** A (recommended) or B above.
2. **Who sees a record.** Recommend: a person sees their own record on a game's page; a public
   record (by handle, a "most wins" list) waits for a profile page, since a public one is a privacy
   choice that people should make. The alternative is public from the start.
3. **What counts toward a record.** Recommend: finished matches with at least two different people,
   all variants of a game together with a breakdown by variant, a resignation is a loss, a match
   nobody finished is not counted, and a person who played as a guest keeps what they played when they
   sign in (matches already follow the account). In games with more than two seats, a win is a win and
   everyone else loses.
4. **Who can post.** Recommend: signed-in accounts that have chosen a handle. Guests read only.
5. **Which games get a community.** Recommend: every game page, catalogue or user-made, with variants
   sharing the page's. A game that is unpublished or removed keeps its discussion hidden with it.
