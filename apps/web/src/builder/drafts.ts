/** The games being written in the workbench. They live in this browser, and a signed-in person's
 *  can also be kept on their account (`sync.ts` says how a draft stands with it). Signed out, nothing
 *  is sent anywhere, so clearing site data takes them (the page says so, and offers a download). */

import { LOCAL, type Sync, resume } from "./sync";

export interface Draft {
  id: string;
  name: string;
  /** The game's spec, as text: what is being written, valid or not. */
  rules: string;
  /** The board's look, as text. Empty is "no skin": the board's own default look. */
  skin: string;
  updated: number;
  /** The id of its copy on the account, once there is one. */
  remote?: string;
  /** How it stands with the account. */
  sync: Sync;
  /** The secret of its private link, while it is shared. */
  shared: string | null;
}

export interface Drafts {
  drafts: Draft[];
  /** The draft on show, when there is one. */
  current: string | null;
}

const KEY = "hubbable.drafts";

const SYNC_KINDS = ["local", "synced", "dirty", "saving", "failed", "refused", "conflict"];

function readSync(value: unknown): Sync {
  if (typeof value !== "object" || value === null) return LOCAL;
  const state = value as { kind?: unknown; version?: unknown; current?: unknown };
  if (typeof state.kind !== "string" || !SYNC_KINDS.includes(state.kind)) return LOCAL;
  if (state.kind === "local") return LOCAL;
  const versionOk = state.version === null || typeof state.version === "number";
  if (!versionOk) return LOCAL;
  if ((state.kind === "synced" || state.kind === "conflict") && state.version === null) return LOCAL;
  if (state.kind === "conflict" && typeof state.current !== "number") return LOCAL;
  return resume(value as Sync);
}

/** A draft as kept, or nothing when it is not one. Drafts kept before the account existed have no
 *  sync and no link: they are local. A save that was on its way is due again. */
function readDraft(value: unknown): Draft | null {
  if (typeof value !== "object" || value === null) return null;
  const draft = value as Record<string, unknown>;
  if (
    typeof draft.id !== "string" ||
    typeof draft.name !== "string" ||
    typeof draft.rules !== "string" ||
    typeof draft.skin !== "string" ||
    typeof draft.updated !== "number"
  ) {
    return null;
  }
  const remote = typeof draft.remote === "string" ? draft.remote : undefined;
  const read = readSync(draft.sync);
  // A draft with no copy on the account is only here, unless it is waiting to be put there.
  const waiting = read.kind === "dirty" && read.version === null;
  const sync = remote !== undefined || waiting ? read : LOCAL;
  return {
    id: draft.id,
    name: draft.name,
    rules: draft.rules,
    skin: draft.skin,
    updated: draft.updated,
    remote,
    sync,
    shared: typeof draft.shared === "string" ? draft.shared : null,
  };
}

/** What the browser kept, or nothing at all. Anything that does not look like a draft is left out. */
export function loadDrafts(): Drafts {
  try {
    const saved: unknown = JSON.parse(localStorage.getItem(KEY) ?? "null");
    if (typeof saved === "object" && saved !== null && Array.isArray((saved as Drafts).drafts)) {
      const drafts = (saved as Drafts).drafts.map(readDraft).filter((d): d is Draft => d !== null);
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

/** A new draft. `sync` is `{ kind: "dirty", version: null }` for one that is to go to the account. */
export function newDraft(name: string, rules: string, skin = "", sync: Sync = LOCAL): Draft {
  return { id: freshId(), name, rules, skin, updated: Date.now(), sync, shared: null };
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
