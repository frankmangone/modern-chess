// The shapes the two APIs answer with. They mirror the Rust responses in
// crates/game/src/http and crates/social/src/http; change them together.

export interface CatalogEntry {
  slug: string;
  name: string;
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
  capacity: number;
  players: LobbyPlayer[];
  created_at: string;
}

export interface GameDetail {
  page: GamePage;
  spec: {
    name: string;
    version: number;
    seats: string[];
    rules: unknown;
  };
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
  hands: boolean;
}

/** A match as the server shows it (crates/game/src/http/play.rs). `you` is the caller's seat. */
export interface InstanceState {
  id: string;
  slug: string;
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
  | { type: "promote"; code: string };
