import type { GameDetail } from "../api/types";

/** The game's "how to play" text, or null when its page has none. */
export function howToPlay(detail: GameDetail): string | null {
  for (const section of detail.page.sections) {
    if (section.type === "how_to_play") return section.markdown;
  }
  return null;
}

/** The first paragraph of a text: enough to say what the game is before the full rules. */
export function firstParagraph(markdown: string): string {
  return markdown.split(/\n\s*\n/)[0]?.trim() ?? "";
}

/** Whether the page lists a section of this type. A game page puts each type in its place: the
 *  preview is the board by the title, the rules have a page of their own, the lobbies are the
 *  play section. */
export function hasSection(detail: GameDetail, type: GameDetail["page"]["sections"][number]["type"]): boolean {
  return detail.page.sections.some((section) => section.type === type);
}

/** "Try it" runs the preview variant: its own skin when it has one, else the game's. */
export function previewSkin(detail: GameDetail): Record<string, unknown> | null {
  return detail.variants.find((v) => v.slug === detail.preview_variant)?.skin ?? detail.page.skin;
}
