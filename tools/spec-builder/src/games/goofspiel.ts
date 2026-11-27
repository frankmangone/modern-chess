// Goofspiel, the Game of Pure Strategy: thirteen prizes turned over one at a time, and each
// round both players bid a card, in secret and at once, for the prize on offer.
import type { Game } from "../build.ts";
import { playingCard } from "../art/faces.ts";
import { bid, biddingBags, biddingEnds, biddingScore, rankCode, rankLabel, reveal, round, type Bidding } from "../recipes/sealedBids.ts";
import { gridBoard, gridPlayers, piece, standardSections, turns } from "../scaffold.ts";

const goofspiel: Bidding = { players: ["RED", "BLUE"], ranks: 13 };
const ranks = Array.from({ length: goofspiel.ranks }, (_, i) => i + 1);

const howToPlay = [
  "Goofspiel is a game of nerve. Thirteen **prize cards** lie shuffled and face down, and each of you holds the same thirteen cards, **A to K**, as your bids. A card is worth its rank: the ace is 1, the jack 11, the queen 12, the king 13.",
  "",
  "- **A round.** The top prize is turned over. Both of you choose one card from your hand and play it face down, **at the same time**: neither sees the other's until both are in.",
  "- **Turning over.** The player who started the round turns the bids over. The **higher card** wins the prize. If the two cards are equal, nobody wins it and it is thrown out.",
  "- **Spent.** A bid card is gone for good. Your hand shrinks every round, and the other player can always see what you have left.",
  "- **Winning.** After thirteen rounds, the player whose prizes add up to more wins.",
  "",
  "Nothing is hidden but the bid in play and the prizes still to come, so the game is all about guessing: spend a king on a two and you have thrown it away; bid low on a big prize and you give it up.",
].join("\n");

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
  page: {
    slug: "goofspiel",
    name: "Goofspiel",
    tagline: "Bid in secret for the prize on the table. A king on a two is a king wasted.",
    description:
      "The Game of Pure Strategy, for two. Prizes are turned over one at a time, and both players play a card face down for each, at the same time. The higher card takes the prize. You hold the same cards, so it is all about reading the other player.",
    tags: ["card game", "two players", "bluffing"],
    accent: "#a63d40",
    skin: {
      captured_panel: false,
      colours: { RED: "#a63d40", BLUE: "#2f5d8f" },
      // The table: the deck and the prize on offer, then each player's bid. A bid made and not yet
      // turned over lies face down, and flips when the bids are turned over.
      table: {
        slots: [
          { label: "Prizes", bag: "DECK", pile: true },
          { label: "Prize", bag: "PRIZE" },
          { label: "RED", bag: "LAST", of: "RED", bid: "PICK" },
          { label: "BLUE", bag: "LAST", of: "BLUE", bid: "PICK" },
        ],
      },
      labels: {
        // What the table shows is left out of the side panel: it keeps the hands, a card played
        // face down (only its player sees it there) and what each has won.
        order: ["HAND", "PICK", "WON"],
        hide_empty: ["PICK"],
        bags: { DECK: false, PRIZE: false, LAST: false, PICK: "{owner} bid face down", WON: "{owner} prizes" },
        actions: { ROUND: "Turn up the next prize", REVEAL: "Turn the bids over" },
        hints: {
          held: "Choose a card to bid, face down.",
          ROUND: "Turn up the next prize, then both of you bid.",
          REVEAL: "Both bids are in. Turn them over.",
        },
        piece_codes: false,
      },
      pieces: Object.fromEntries(ranks.map((rank) => [rankCode(rank), { svg: playingCard(rankLabel(rank, goofspiel.ranks)) }])),
    },
    sections: standardSections(howToPlay),
    featured: false,
    sortOrder: 22,
  },
  files: { spec: "crates/engine/specs/goofspiel.json", page: "crates/game/pages/goofspiel.json" },
};
