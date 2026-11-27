/** The games being written in the workbench. They live in this browser only: nothing here is sent
 *  anywhere, so clearing site data takes them (the page says so, and offers a download). */

export interface Draft {
  id: string;
  name: string;
  /** The game's spec, as text: what is being written, valid or not. */
  rules: string;
  /** The board's look, as text. Empty is "no skin": the board's own default look. */
  skin: string;
  updated: number;
}

export interface Drafts {
  drafts: Draft[];
  /** The draft on show, when there is one. */
  current: string | null;
}

const KEY = "hubbable.drafts";

function isDraft(value: unknown): value is Draft {
  if (typeof value !== "object" || value === null) return false;
  const draft = value as Record<string, unknown>;
  return (
    typeof draft.id === "string" &&
    typeof draft.name === "string" &&
    typeof draft.rules === "string" &&
    typeof draft.skin === "string" &&
    typeof draft.updated === "number"
  );
}

/** What the browser kept, or nothing at all. Anything that does not look like a draft is left out. */
export function loadDrafts(): Drafts {
  try {
    const saved: unknown = JSON.parse(localStorage.getItem(KEY) ?? "null");
    if (typeof saved === "object" && saved !== null && Array.isArray((saved as Drafts).drafts)) {
      const drafts = (saved as Drafts).drafts.filter(isDraft);
      const current = (saved as Drafts).current;
      return { drafts, current: drafts.some((d) => d.id === current) ? current : (drafts[0]?.id ?? null) };
    }
  } catch {
    // Nothing kept, or storage is shut: start empty.
  }
  return { drafts: [], current: null };
}

/** Whether the browser took it. A full or shut store says no, and the page says so. */
export function saveDrafts(drafts: Drafts): boolean {
  try {
    localStorage.setItem(KEY, JSON.stringify(drafts));
    return true;
  } catch {
    return false;
  }
}

function freshId(): string {
  return globalThis.crypto?.randomUUID?.() ?? `${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`;
}

export function newDraft(name: string, rules: string, skin = ""): Draft {
  return { id: freshId(), name, rules, skin, updated: Date.now() };
}

/** `name` made different from every name in `taken`: "Chess", "Chess 2", "Chess 3". */
export function uniqueName(name: string, taken: string[]): string {
  if (!taken.includes(name)) return name;
  for (let n = 2; ; n++) {
    if (!taken.includes(`${name} ${n}`)) return `${name} ${n}`;
  }
}

/** A name that works as a file name: the letters and numbers of it, joined by dashes. */
export function fileName(name: string): string {
  const slug = name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return `${slug || "game"}.json`;
}
