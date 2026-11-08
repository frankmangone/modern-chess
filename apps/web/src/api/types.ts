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
