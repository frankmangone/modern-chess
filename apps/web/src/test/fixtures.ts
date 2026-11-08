import type { CatalogEntry } from "../api/types";

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
