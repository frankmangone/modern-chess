import type { BoardView, CatalogEntry, GameDetail, InstanceState, PageSection } from "../api/types";

export function entry(overrides: Partial<CatalogEntry> = {}): CatalogEntry {
  return {
    slug: "chess",
    name: "Chess",
    tagline: "The classic duel of kings.",
    tags: ["classic", "two players"],
    accent: "#8a5a2b",
    cover_image: null,
    featured: false,
    ...overrides,
  };
}

export const FOUR_GAMES: CatalogEntry[] = [
  entry({ slug: "chess", name: "Chess", featured: true }),
  entry({ slug: "checkers", name: "Checkers", tagline: "Jump to win.", accent: "#b03a2e" }),
  entry({ slug: "ataxx", name: "Ataxx", tagline: "Clone and conquer.", accent: "#2e86c1" }),
  entry({ slug: "chaturanga", name: "Chaturanga", tagline: "The ancestor of chess.", accent: null }),
];

export const CHESS_SECTIONS: PageSection[] = [
  { type: "how_to_play", markdown: "Capture the **king**." },
  { type: "preview" },
  { type: "open_lobbies" },
];

export function detail(overrides: Partial<GameDetail> = {}): GameDetail {
  return {
    page: {
      slug: "chess",
      tagline: "The classic duel of kings.",
      description: "Two armies on sixty-four squares.",
      tags: ["classic", "two players"],
      accent: "#8a5a2b",
      cover_image: null,
      skin: { pieces: { KING: "K" } },
      sections: CHESS_SECTIONS,
      featured: true,
      sort_order: 0,
    },
    spec: { name: "Chess", version: 1, seats: ["white", "black"], rules: { name: "Chess", board: {} } },
    open_lobbies: [
      {
        id: "11111111-1111-1111-1111-111111111111",
        capacity: 2,
        players: [{ seat: "white", handle: "Guest-1111" }],
        created_at: "2026-11-08T10:00:00Z",
      },
    ],
    ...overrides,
  };
}

/** A token the way social-api makes one, as far as the browser reads it: only `exp` matters. */
export function fakeToken(expiresInSeconds: number): string {
  const encode = (value: unknown) =>
    btoa(JSON.stringify(value)).replaceAll("+", "-").replaceAll("/", "_").replaceAll("=", "");
  const exp = Math.floor(Date.now() / 1000) + expiresInSeconds;
  return `${encode({ alg: "EdDSA" })}.${encode({ exp })}.signature`;
}

export function boardView(overrides: Partial<BoardView> = {}): BoardView {
  return {
    width: 3,
    height: 3,
    disabled: [],
    players: ["WHITE", "BLACK"],
    pieces: [
      { x: 0, y: 0, code: "ROOK", player: "WHITE" },
      { x: 2, y: 2, code: "ROOK", player: "BLACK" },
    ],
    turn: "WHITE",
    selected: null,
    destinations: [],
    promotion: null,
    status: { state: "playing" },
    zones: {},
    hands: false,
    hand: {},
    drops: {},
    captured: {},
    material: {},
    supply: [],
    can_pass: false,
    passes: 0,
    score: {},
    ...overrides,
  };
}

export function snapshot(overrides: Partial<InstanceState> = {}): InstanceState {
  return {
    id: "33333333-3333-3333-3333-333333333333",
    slug: "chess",
    status: "active",
    revision: 3,
    capacity: 2,
    seats: ["WHITE", "BLACK"],
    players: [
      { seat: "WHITE", handle: "Guest-1" },
      { seat: "BLACK", handle: "Guest-2" },
    ],
    created_at: "2026-11-08T10:00:00Z",
    started_at: "2026-11-08T10:01:00Z",
    view: boardView(),
    outcome: null,
    you: "WHITE",
    ...overrides,
  };
}
