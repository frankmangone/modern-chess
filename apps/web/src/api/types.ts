// The shapes the two APIs answer with. They mirror the Rust responses in
// crates/game/src/http and crates/social/src/http; change them together.

export interface VariantLabel {
  slug: string;
  label: string;
}

/** One card on the front page: a game, with the variants it can be played in. */
export interface CatalogEntry {
  slug: string;
  name: string;
  variants: VariantLabel[];
  tagline: string;
  tags: string[];
  accent: string | null;
  cover_image: string | null;
  /** The family the front page groups the game under ("Chess and its variants"), if any. */
  family: string | null;
  featured: boolean;
}

export type PageSection =
  | { type: "how_to_play"; markdown: string }
  | { type: "preview" }
  | { type: "open_lobbies" };

export interface GamePage {
  slug: string;
  /** The game's own name ("Go"). The server always fills it in on a game's page. */
  name: string;
  /** The variant "Try it" runs. */
  preview_variant: string;
  tagline: string;
  description: string;
  tags: string[];
  accent: string | null;
  cover_image: string | null;
  skin: Record<string, unknown> | null;
  sections: PageSection[];
  family: string | null;
  featured: boolean;
  sort_order: number;
}

export interface LobbyPlayer {
  seat: string;
  handle: string;
}

export interface LobbySummary {
  id: string;
  /** The variant the lobby will be played in: its slug and what a picker calls it. */
  variant: string;
  variant_label: string;
  capacity: number;
  players: LobbyPlayer[];
  created_at: string;
}

export interface VariantDetail {
  slug: string;
  label: string;
  /** The name of the variant's rules ("Go 13x13"). */
  name: string;
  seats: string[];
  /** A skin that replaces the game's, or null when the variant uses the game's. */
  skin: Record<string, unknown> | null;
}

export interface GameDetail {
  page: GamePage;
  /** The preview variant's rules, which "Try it" runs. */
  spec: {
    name: string;
    version: number;
    seats: string[];
    rules: unknown;
  };
  preview_variant: string;
  variants: VariantDetail[];
  /** Open lobbies of every variant of the game, newest first. */
  open_lobbies: LobbySummary[];
}

export interface Guest {
  user: { id: string; handle: string };
  token: string;
}

export type InstanceStatus = "lobby" | "active" | "finished";

/** `reason` is absent for a win by play. */
export type Outcome = { type: "won"; seat: string; reason?: "resigned" } | { type: "draw" };

/** A place on the board: the name a click answers with, and where to draw it. On a grid `at` is
 *  the cell; on a map it may be fractional, and the node says what kind of place it is. */
export interface BoardNode {
  name: string;
  at: [number, number];
  /** Map places only. */
  kind?: "point" | "path" | "area";
  /** A path's two points. */
  ends?: [string, string];
  /** An area's points, in drawing order. */
  around?: string[];
}

/** One answer to a choice: exactly one of a place's name, a code, a held piece or an action's name. */
export type Answer = { place: string } | { code: string } | { held: string } | { action: string };

/** What a place answers: a piece to pick, a square to move to or a point to place on. */
export type ChoiceKind = "PIECE" | "MOVE" | "PLACE";

/** The choice in front of the player to move (crates/engine/src/logic/play/ways/choice.rs). */
export interface Choice {
  /** The answers given so far this turn. */
  chosen: Answer[];
  /** The chosen piece's place, drawn as selected. */
  start: string | null;
  /** Each place that answers the next choice, with the kind of choice it answers. */
  places: { name: string; for: ChoiceKind }[];
  /** A promotion, or any code choice. */
  codes: string[];
  /** The pieces held in a bag that can be chosen. */
  held: string[];
  /** Actions to name now; one that `plays` is played at once (a pass), another only narrows the ways. */
  actions: { name: string; plays: boolean }[];
}

/** A bag of pieces: a player's hand (the bag HAND) or a store of the game's. `owner` is absent for a
 *  shared bag; `pieces` is piece code, then how many it holds. A bag the viewer may not see into has
 *  `hidden` set and no pieces, and a `count` when the game lets everyone see how many it keeps. */
export interface BagView {
  name: string;
  owner?: string;
  pieces: Record<string, number>;
  /** The viewer may not see what it keeps (the pieces are empty). Absent when they may. */
  hidden?: boolean;
  /** How many pieces a hidden bag keeps, when its count shows. Absent for a bag the viewer sees into. */
  count?: number;
}

/** What the board draws: the JSON the engine reports (crates/session/src/view.rs). Positions are
 *  node names; only `nodes` says where each one is drawn. */
export interface BoardView {
  /** Square or hex cells, or a map's places. */
  shape: "square" | "hex" | "map";
  /** On a hex board: a wide diamond (`rhombus`) or an upright `hexagon`. */
  layout?: "rhombus" | "hexagon";
  /** The board's nodes in board order; a cell the board does not have is not listed. */
  nodes: BoardNode[];
  /** On a map: the written links between points that no path place draws. */
  lines?: [string, string][];
  players: string[];
  /** `count` is set for pieces kept in a place that holds many (a pit of Kalah); no `count` means one
   *  piece. A `player` of "" means the piece belongs to nobody. */
  pieces: { position: string; code: string; player: string; count?: number }[];
  turn: string;
  /** What the player to move can answer next. Empty for everyone else. */
  choice: Choice;
  status: { state: "playing" } | { state: "draw" } | { state: "won"; winner: string };
  zones: Record<string, Record<string, string[]>>;
  /** The bags of the game, sorted by name then owner. A player's hand is the bag named HAND with that
   *  owner, and every player has one in a hand game. Empty when the game has no bags. */
  bags: BagView[];
  /** Player, then piece code, then how many of that player's pieces have been lost. */
  captured: Record<string, Record<string, number>>;
  /** Player, then what their pieces are worth, hand included. Empty when the game has no values. */
  material: Record<string, number>;
  /** How many turns in a row, up to now, were passes. */
  passes: number;
  /** Player, then score as if the game ended now. Empty when the game does not score by area. */
  score: Record<string, number>;
}

/** A match as the server shows it (crates/game/src/http/play.rs). `you` is the caller's seat. */
export interface InstanceState {
  id: string;
  /** The variant being played: the slug of its rules. */
  slug: string;
  /** The game the variant belongs to. */
  game: string;
  /** Null when the server does not know the variant. */
  variant_label: string | null;
  status: InstanceStatus;
  revision: number;
  capacity: number;
  seats: string[];
  players: LobbyPlayer[];
  created_at: string;
  started_at: string | null;
  view: BoardView;
  outcome: Outcome | null;
  you: string | null;
}

export type Command = ({ type: "choose" } & Answer) | { type: "cancel" } | { type: "resign" };
