// Sealed bids for prizes (Goofspiel, the Game of Pure Strategy): a shuffled pile of prizes, a hand
// of bidding cards for each player that everyone can see, and a bid that each player makes in
// secret and at once. The higher bid takes the prize; a tie takes it out of the game.
//
// Every card is a piece whose code is its rank, `C01` to `C13`, so the codes sort in rank order.
// The cards a player holds (HAND), their face-down bid (PICK, seen by the bidder alone) and the bid
// just turned over (LAST) are bags of each player; the prizes wait in DECK (a hidden pile), and
// the one on offer lies in PRIZE.
import {
  action, all, ask, atBag, bag, chooseHeld, codes, count, draw, moveAll, moveHeld, remove, shuffle, when,
  type Action, type BagRef, type Step, type Test, type Where,
} from "../blocks.ts";
import { bagDef, baggedTerm, endCheck, type BagDef, type EndCheck, type ScoreTerm } from "../scaffold.ts";

export interface Bidding {
  /** The two players, in turn order. */
  players: [string, string];
  /** How many ranks each player bids: 13 is the classic game. */
  ranks: number;
}

export const THROW = "THROW";
export const REVEAL = "REVEAL";

/** The code of a card of rank `rank` (from 1): `C01`. Two digits so that codes sort by rank. */
export function rankCode(rank: number): string {
  if (!Number.isInteger(rank) || rank < 1) throw new Error(`rankCode: ranks count from 1 (got ${rank})`);
  return `C${String(rank).padStart(2, "0")}`;
}

/** What a card is called on the table: A, 2 to 10, J, Q, K for the classic thirteen, a number
 *  otherwise. */
export function rankLabel(rank: number, ranks: number): string {
  if (ranks === 13) return ({ 1: "A", 11: "J", 12: "Q", 13: "K" } as Record<number, string>)[rank] ?? String(rank);
  return String(rank);
}

const range = (from: number, to: number): number[] => Array.from({ length: Math.max(0, to - from + 1) }, (_, i) => from + i);
const everyRank = ({ ranks }: Bidding): Record<string, number> => Object.fromEntries(range(1, ranks).map((rank) => [rankCode(rank), 1]));

function checkBidding({ players, ranks }: Bidding): void {
  if (players[0] === players[1]) throw new Error("sealedBids: two different players are needed");
  if (!Number.isInteger(ranks) || ranks < 2) throw new Error(`sealedBids: at least 2 ranks are needed (got ${ranks})`);
}

/** The bags of the game. The deck is hidden from everyone but its height is told, and no rule may
 *  read what it holds, so no way to play can give a prize away. */
export function biddingBags(bidding: Bidding): BagDef[] {
  checkBidding(bidding);
  const cards = everyRank(bidding);
  return [
    bagDef("HAND", { eachPlayer: true, start: cards }),
    bagDef("PICK", { eachPlayer: true, visible: "OWNER" }),
    bagDef("LAST", { eachPlayer: true }),
    bagDef("WON", { eachPlayer: true }),
    bagDef("DECK", { ordered: true, start: cards, visible: "NONE", showsCount: true, strict: true }),
    bagDef("PRIZE"),
  ];
}

const mover = (name: string): BagRef => bag(name, "MOVER");
const enemy = (name: string): BagRef => bag(name, "ENEMY");

/** A round: the prize on top of the deck is turned over, then both players are asked for a bid,
 *  and when both have answered the player who asked turns the bids over. The deck is shuffled
 *  before each draw, which changes nothing about what comes up (nobody has seen the deck) and
 *  spares the spec a "first round". */
export function round(): Action {
  return action("ROUND", [
    count(atBag("DECK"), undefined, { min: 1 }),
    shuffle(bag("DECK")),
    draw(bag("DECK"), bag("PRIZE"), 1),
    ask("ALL", { stage: THROW, together: { sealed: true }, then: REVEAL }),
  ]);
}

/** A bid: one card from your own hand, face down. */
export function bid(): Action {
  return action("BID", [chooseHeld(mover("HAND")), moveHeld(mover("PICK"))], THROW);
}

/** The side whose card is higher: the mover's bid is `rank` and the other's is a lower one. */
function beats(first: BagRef, second: BagRef, rank: number): Test {
  return all(
    count(atBag(first.bag, first.of), codes(rankCode(rank)), { min: 1 }),
    count(atBag(second.bag, second.of), codes(...range(1, rank - 1).map(rankCode)), { min: 1 }),
  );
}

/** Turning the bids over: the higher bid takes the prize, a tie sends it out of the game, and
 *  both bids lie face up until the next round. Nothing here asks the players anything. */
export function reveal({ ranks }: Bidding): Action {
  const awards: Step[] = [];
  for (const [winner, loser] of [[mover("PICK"), enemy("PICK")], [enemy("PICK"), mover("PICK")]] as const) {
    for (const rank of range(2, ranks)) {
      awards.push(when(beats(winner, loser, rank), [moveAll(bag("PRIZE"), bag("WON", winner.of))]));
    }
  }
  return action(
    "REVEAL",
    [
      remove(mover("LAST"), "ALL"),
      remove(enemy("LAST"), "ALL"),
      ...awards,
      remove(bag("PRIZE"), "ALL"),
      moveAll(mover("PICK"), mover("LAST")),
      moveAll(enemy("PICK"), enemy("LAST")),
    ],
    REVEAL,
  );
}

/** The game is over when the last prize has been taken or thrown out. */
export function biddingEnds(): EndCheck[] {
  return [endCheck(all(count(atBag("DECK"), undefined, { min: 0, max: 0 }), count(atBag("PRIZE"), undefined, { min: 0, max: 0 })), "HIGHEST")];
}

/** A prize is worth its rank: a point for each pip, one term each. */
export function biddingScore({ ranks }: Bidding): ScoreTerm[] {
  const terms: ScoreTerm[] = [];
  for (const rank of range(1, ranks)) {
    const only: Where = codes(rankCode(rank));
    for (let point = 0; point < rank; point += 1) terms.push(baggedTerm("WON", only));
  }
  return terms;
}
