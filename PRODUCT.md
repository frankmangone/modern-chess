# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users
Two audiences, about equally. Casual players who want to pick a game, send a link and play a friend without signing up. Board-game enthusiasts who browse for variants and unusual games (Shogi, Grand Chess, Hex, Kalah, Backgammon) and want to read and try the rules before committing. A casual player must get into a game in one step; an enthusiast must be able to find a game and understand its rules.

## Product Purpose
BoardHub is a web hub where board games are played online against a friend. Every game, chess and its variants, Go, Hex, Checkers, Kalah, Snakes and Ladders, Backgammon, is a data spec played by one engine, so adding a game or a variant is cheap. Success: a visitor is playing a game within a minute of arriving, and an enthusiast finds the odd game they came for.

## Positioning
The pitch: **the home of games that are just rules.** Also written: *games as data*, *build your own game*. Use one of these, not a feature list.

- Every game is a spec, not custom code: one engine, many games.
- Boards run the real engine in the browser, so a game can be tried before a lobby is opened.
- A guest name is made the first time someone plays; no sign-up.
- Direction (not built): communities around user-proposed games, made with a builder.

### Scope: not tied to boards
The product is for any game whose rules can be written down: boards, but also dice (Backgammon, Snakes and Ladders), sealed simultaneous choices (Rock Paper Scissors) and cards (the engine has shuffled piles, hands only their owner sees and sealed simultaneous bids; Goofspiel is built as a spec to prove it, and is deliberately not in the catalogue: the hub is not aiming to be a card-game site, though a game builder could write one). "Board" in the product name and the copy is therefore a limit we are leaving; see Naming.

### Landscape and how we differ
What the closest products are, from public pages read in November 2026. Most sources were reviews and aggregator pages, not official documentation, so check a claim before putting it in public copy.

| Neighbour | What it is | Where we differ |
|-----------|------------|-----------------|
| Tabletopia, Tabletop Simulator | A sandbox virtual table with licensed commercial games. Rules are mostly managed by the players. | We run the rules: the engine knows every legal move, turn and ending. We do not simulate a table or sell licensed hits. |
| Board Game Arena | Automated rules, a large catalogue, owned by a publisher (Asmodee), with a premium tier. Each game is coded on its own. | A game here is a small readable file, so the long tail (traditional, historical and invented games) costs little to add. No publisher in the way, no premium tier. |
| PyChess, PlayStrategy, Lishogi, Jocly | Chess variants and Asian games, each hand-coded (mostly on Lichess code), with accounts. | One engine for chess variants, Go, Hex, Kalah, dice and cards together, and a way to write your own. No account to start. |
| Ludii | A general game system: games written in a rules description language. Closest to us in idea. A research and desktop tool; we could not confirm online play. | The same idea made web-native: play a friend from a link in the browser, with no install, and a modern interface. |

What is ours, in order of how visible it is today:
1. **The long tail.** Odd and traditional games (Shogi, Grand Chess, Hex, Kalah, Chaturanga) next to the familiar ones, in one place. Visible now.
2. **No friction.** A visitor can try a board and play a friend from a link with no sign-up. Visible now.
3. **Games as data.** One engine, rules enforced for every game, a spec anyone can read (the docs book teaches it). Visible to builders now, to players through the catalogue.
4. **Authoring.** Anyone, or an agent, writes a game and it is playable with rules enforced. The spec builder (`tools/spec-builder`) is its first step; a hub-side builder and communities around proposed games are not built. This is the strongest claim and the one that is least finished.

What we do not compete on: licensed modern hits, 3D sandbox play with physical components, and the size of the catalogue of the big platforms.

Honest gaps: a small catalogue; the audience for the long tail is real but niche; play is against a friend you invite; the builder is a direction, not a product.

### Naming (open)
"BoardHub" is under review because the product is not tied to boards. Decided: keep **Hub**; the name must not sound like Tabletopia; it should lean on *rules as data / build your own game* and fit the Walnut world (warm, parlour). Considered and set aside: TableHub (too close to Tabletopia), TokenHub (crypto), GameHub and PlayHub (too generic), MeepleHub and PawnHub (still board-shaped). Remaining: ParlourHub (parlour games are the old category for cards, boards and dice; spell-check both parlour and parlor), PipHub (short, but `pip` is Python's package manager), VariantHub (plain, a little chess-flavoured). Before choosing, check the domain, social handles and trademarks: searches found no site under these names, which says little. Renaming touches `apps/web` (about 9 files), this file, the docs book title ("Building Board Games") and copy that says "board game", such as the front page headline. It is cheap; the engine and crate names stay.

## Capabilities and Constraints
- Game pages are a list of sections from the game's page data (how to play, preview, open lobbies); no engine or server change is part of a design pass.
- Each game has its own accent colour in the catalogue.
- Games with dice (Snakes and Ladders, Backgammon) roll in one action; the view shows no roll for Snakes and Ladders today (to check whether it can).
- Frontend: Solid + Vite (`apps/web`); the board element is shared with the docs book (`apps/docs/theme`); the engine runs as wasm.
- The docs book is served at `/docs/`; it is for people who build games and is not a priority.

## Brand Commitments
- The Walnut world (walnut ground, paper surfaces, thick dark outlines) and its buttons: outlined, with a hard shadow they press into. The buttons are liked and stay.
