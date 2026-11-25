import type {
  BoardNode,
  BoardView,
  CatalogEntry,
  Choice,
  GameDetail,
  InstanceState,
  PageSection,
  VariantDetail,
} from "../api/types";

export function entry(overrides: Partial<CatalogEntry> = {}): CatalogEntry {
  return {
    slug: "chess",
    name: "Chess",
    variants: [{ slug: "chess", label: "Chess" }],
    tagline: "The classic duel of kings.",
    tags: ["classic", "two players"],
    accent: "#8a5a2b",
    skin: null,
    parent: null,
    featured: false,
    board: null,
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
      name: "Chess",
      preview_variant: "chess",
      tagline: "The classic duel of kings.",
      description: "Two armies on sixty-four squares.",
      tags: ["classic", "two players"],
      accent: "#8a5a2b",
      cover_image: null,
      skin: { pieces: { KING: "K" } },
      sections: CHESS_SECTIONS,
      parent: null,
      featured: true,
      sort_order: 0,
    },
    board: null,
    parent: null,
    children: [],
    spec: { name: "Chess", version: 1, seats: ["white", "black"], rules: { name: "Chess", board: {} } },
    preview_variant: "chess",
    variants: [
      { slug: "chess", label: "Chess", name: "Chess", seats: ["white", "black"], skin: null },
    ],
    open_lobbies: [
      {
        id: "11111111-1111-1111-1111-111111111111",
        variant: "chess",
        variant_label: "Chess",
        capacity: 2,
        players: [{ seat: "white", handle: "Guest-1111" }],
        created_at: "2026-11-08T10:00:00Z",
      },
    ],
    ...overrides,
  };
}

export const GO_SKIN_13 = { board: { style: "intersections", dots: [[3, 3]] } };

/** Go: one game, three boards. The 9x9 uses the game's skin; the 13x13 has its own. */
export const GO_VARIANTS: VariantDetail[] = [
  { slug: "go", label: "9x9", name: "Go 9x9", seats: ["BLACK", "WHITE"], skin: null },
  { slug: "go-13", label: "13x13", name: "Go 13x13", seats: ["BLACK", "WHITE"], skin: GO_SKIN_13 },
  { slug: "go-19", label: "19x19", name: "Go 19x19", seats: ["BLACK", "WHITE"], skin: null },
];

export function goDetail(overrides: Partial<GameDetail> = {}): GameDetail {
  const base = detail();
  return {
    ...base,
    page: {
      ...base.page,
      slug: "go",
      name: "Go",
      preview_variant: "go",
      skin: { board: { style: "intersections" } },
    },
    spec: { name: "Go 9x9", version: 1, seats: ["BLACK", "WHITE"], rules: { name: "GO", board: {} } },
    preview_variant: "go",
    variants: GO_VARIANTS,
    open_lobbies: [],
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

/** The nodes of a plain grid, named as the engine names them: "x,y", drawn at [x, y]. */
export function gridNodes(width: number, height: number): BoardNode[] {
  const nodes: BoardNode[] = [];
  for (let x = 0; x < width; x++) {
    for (let y = 0; y < height; y++) nodes.push({ name: `${x},${y}`, at: [x, y] });
  }
  return nodes;
}

/** The choice of a player with nothing to answer: what everyone but the player to move sees. */
export function choice(overrides: Partial<Choice> = {}): Choice {
  return { chosen: [], start: null, places: [], codes: [], held: [], actions: [], ...overrides };
}

export function boardView(overrides: Partial<BoardView> = {}): BoardView {
  return {
    shape: "square",
    nodes: gridNodes(3, 3),
    players: ["WHITE", "BLACK"],
    pieces: [
      { position: "0,0", code: "ROOK", player: "WHITE" },
      { position: "2,2", code: "ROOK", player: "BLACK" },
    ],
    turn: "WHITE",
    choice: choice({ places: [{ name: "0,0", for: "PIECE" }] }),
    status: { state: "playing" },
    zones: {},
    bags: [],
    captured: {},
    material: {},
    passes: 0,
    score: {},
    ...overrides,
  };
}

export function snapshot(overrides: Partial<InstanceState> = {}): InstanceState {
  return {
    id: "33333333-3333-3333-3333-333333333333",
    slug: "chess",
    game: "chess",
    variant_label: "Chess",
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
