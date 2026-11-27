// Goofspiel, the Game of Pure Strategy: thirteen prizes turned over one at a time, and each
// round both players bid a card, in secret and at once, for the prize on offer. It has no page: it
// is not in the catalogue, but it shows (and its session tests prove) that a card game with hidden
// piles and sealed bids can be written as a spec.
import type { Game } from "../build.ts";
import { bid, biddingBags, biddingEnds, biddingScore, rankCode, rankLabel, reveal, round, type Bidding } from "../recipes/sealedBids.ts";
import { gridBoard, gridPlayers, piece, turns } from "../scaffold.ts";

const goofspiel: Bidding = { players: ["RED", "BLUE"], ranks: 13 };
const ranks = Array.from({ length: goofspiel.ranks }, (_, i) => i + 1);

export const goofspielGame: Game = {
  spec: {
    name: "Goofspiel",
    board: gridBoard(1, 1),
    players: gridPlayers(...goofspiel.players),
    turns: turns([...goofspiel.players], ["THROW", "REVEAL"]),
    pieces: ranks.map((rank) => piece(rankCode(rank), `card ${rankLabel(rank, goofspiel.ranks)}`)),
    bags: biddingBags(goofspiel),
    actions: [round(), bid(), reveal(goofspiel)],
    endChecks: biddingEnds(),
    score: biddingScore(goofspiel),
  },
  files: { spec: "crates/engine/specs/goofspiel.json" },
};
