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

export type Outcome = { type: "won"; seat: string } | { type: "draw" };

export type Square = [number, number];

/** What the board draws: the JSON the engine reports (crates/session/src/view.rs). */
export interface BoardView {
  width: number;
  height: number;
  disabled: Square[];
  players: string[];
  pieces: { x: number; y: number; code: string; player: string }[];
  turn: string;
  selected: Square | null;
  destinations: Square[];
  promotion: string[] | null;
  status: { state: "playing" } | { state: "draw" } | { state: "won"; winner: string };
  zones: Record<string, Record<string, Square[]>>;
  /** True when captured pieces go to a hand and can be dropped back (Shogi). */
  hands: boolean;
  /** Player, then piece code, then how many they hold. Every player has an entry in a hand game. */
  hand: Record<string, Record<string, number>>;
  /** For the player to move: piece code, then the squares it may be dropped on. */
  drops: Record<string, Square[]>;
  /** Player, then piece code, then how many of that player's pieces have been lost. */
  captured: Record<string, Record<string, number>>;
  /** Player, then what their pieces are worth, hand included. Empty when the game has no values. */
  material: Record<string, number>;
  /** Piece codes any player may place without a hand (the stones in Go). Their squares are in `drops`. */
  supply: string[];
  /** True when the player to move may pass right now. */
  can_pass: boolean;
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

export type Command =
  | { type: "select"; x: number; y: number }
  | { type: "move"; x: number; y: number }
  | { type: "promote"; code: string }
  | { type: "drop"; code: string; x: number; y: number }
  | { type: "pass" };
