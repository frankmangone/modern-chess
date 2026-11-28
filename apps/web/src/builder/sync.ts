import type { Listing } from "../api/types";

/** How a draft stands with the account, as a state machine.
 *
 *  `step` is the one place that knows which state follows which event, and the page and the tests
 *  both read it. The browser always holds the latest text; the account holds the text as of the last
 *  save, and `version` is the version of that copy (null when there is none yet). A save names the
 *  version it starts from, so another browser's work is never silently overwritten. */

export type Sync =
  /** Only in this browser. Signed out, every draft is this. */
  | { kind: "local" }
  /** The account holds what the browser holds. */
  | { kind: "synced"; version: number }
  /** Changed since the last save, which is due soon. */
  | { kind: "dirty"; version: number | null }
  /** A save is on its way. `again` is set when the text changed meanwhile. */
  | { kind: "saving"; version: number | null; again: boolean }
  /** The save could not be made (no connection, say). The work is safe in the browser, and a save is tried again. */
  | { kind: "failed"; version: number | null }
  /** The account refused the text itself, for example because it is too big. Changing it tries again. */
  | { kind: "refused"; version: number | null; reason: string }
  /** The draft was saved from somewhere else too. `current` is the version now on the account. */
  | { kind: "conflict"; version: number; current: number };

export type SyncEvent =
  /** The person changed the text. */
  | { type: "edit" }
  /** The person put a local draft on their account. */
  | { type: "upload" }
  /** The save is due: it is sent. */
  | { type: "send" }
  /** The account kept it, as this version. */
  | { type: "saved"; version: number }
  /** The save could not be made. */
  | { type: "failed" }
  /** The account refused the text. */
  | { type: "refused"; reason: string }
  /** The account says another save came first. */
  | { type: "conflict"; current: number }
  /** Time to try again after a failure. */
  | { type: "retry" }
  /** The person chose to keep their version over the other one. */
  | { type: "keep-mine" }
  /** The person chose the other version, which is now on the page, as this version. */
  | { type: "take-theirs"; version: number }
  /** The account no longer has this draft (deleted elsewhere): what is here is only local now. */
  | { type: "gone" };

export const LOCAL: Sync = { kind: "local" };

/** The state after an event. An event that means nothing in a state leaves it as it is, so a late
 *  answer to an old request cannot move a draft somewhere it does not belong. */
export function step(state: Sync, event: SyncEvent): Sync {
  switch (event.type) {
    case "edit":
      switch (state.kind) {
        case "synced":
          return { kind: "dirty", version: state.version };
        case "failed":
        case "refused":
          return { kind: "dirty", version: state.version };
        case "saving":
          return { ...state, again: true };
        default:
          return state; // local and dirty stay; a conflict stays until it is settled
      }
    case "upload":
      return state.kind === "local" ? { kind: "dirty", version: null } : state;
    case "send":
      return state.kind === "dirty" ? { kind: "saving", version: state.version, again: false } : state;
    case "saved":
      return state.kind === "saving"
        ? state.again
          ? { kind: "dirty", version: event.version }
          : { kind: "synced", version: event.version }
        : state;
    case "failed":
      return state.kind === "saving" ? { kind: "failed", version: state.version } : state;
    case "refused":
      return state.kind === "saving" ? { kind: "refused", version: state.version, reason: event.reason } : state;
    case "conflict":
      // A first save has nothing to conflict with, so only a save of a known version can.
      return state.kind === "saving" && state.version !== null
        ? { kind: "conflict", version: state.version, current: event.current }
        : state;
    case "retry":
      return state.kind === "failed" ? { kind: "dirty", version: state.version } : state;
    case "keep-mine":
      return state.kind === "conflict" ? { kind: "dirty", version: state.current } : state;
    case "take-theirs":
      return state.kind === "conflict" ? { kind: "synced", version: event.version } : state;
    case "gone":
      return state.kind === "local" ? state : LOCAL;
  }
}

/** What a draft read back from the browser should start as: a save that was on its way when the page
 *  closed, or that failed, has to be sent again, and nobody is waiting for it. */
export function resume(state: Sync): Sync {
  switch (state.kind) {
    case "saving":
    case "failed":
    case "refused":
      return { kind: "dirty", version: state.version };
    default:
      return state;
  }
}

/** Whether the account holds (or will hold) a copy: the draft is not only in this browser. */
export function onAccount(state: Sync): boolean {
  return state.kind !== "local";
}

/** What a person is told about it, or null when there is nothing to say (a local draft says so elsewhere). */
export function describe(state: Sync): string | null {
  switch (state.kind) {
    case "local":
      return null;
    case "synced":
      return "Saved to your account";
    case "dirty":
    case "saving":
      return "Saving…";
    case "failed":
      return "Not saved to your account yet. It is kept in this browser and tried again.";
    case "refused":
      return `Not saved to your account: ${state.reason}`;
    case "conflict":
      return "Changed somewhere else too";
  }
}

// ---- The browser's drafts and the account's, put together ------------------------------------

/** What the account says about a draft apart from its text: it comes with the draft, and is the account's
 *  to say, so it is taken as it is whenever the account is read. */
export interface Standing {
  /** The page's words, when the game has been handed in, or kept from the game this revises. */
  listing?: Listing | null;
  /** The id of the published game this draft is a new revision of. */
  revises?: string | null;
  /** Why staff sent the game back, when that was the last thing that happened to it. */
  rejection?: string | null;
}

/** What the account says about a draft, made whole: a missing field is nothing. */
export function standingOf(remote: Standing): Required<Standing> {
  return { listing: remote.listing ?? null, revises: remote.revises ?? null, rejection: remote.rejection ?? null };
}

/** The part of a draft the account sends back. */
export interface Remote extends Standing {
  id: string;
  name: string;
  rules: string;
  skin: string;
  version: number;
  share_token: string | null;
}

/** The part of a draft that this module reads and writes: the rest is the page's. */
export interface Syncable extends Standing {
  name: string;
  rules: string;
  skin: string;
  /** The id on the account, once the account has one. */
  remote?: string;
  sync: Sync;
  /** The secret of the draft's private link, when it is shared. */
  shared: string | null;
}

/** The browser's drafts put together with what the account holds, once the account has been asked.
 *  - An account draft the browser has not seen is added, as saved.
 *  - One that changed on the account while the browser's copy was untouched takes the new text.
 *  - One that changed on both sides is a conflict, and the person chooses.
 *  - One the account no longer has goes if the browser's copy was only a copy of it, and stays as a
 *    local draft if it held changes that were never saved, so no work is lost. */
export function reconcile<D extends Syncable>(local: D[], remote: Remote[], make: (from: Remote) => D): D[] {
  const there = new Map(remote.map((r) => [r.id, r]));
  const had = new Set<string>();
  const out: D[] = [];

  for (const draft of local) {
    if (draft.remote === undefined) {
      out.push(draft);
      continue;
    }
    had.add(draft.remote);
    const theirs = there.get(draft.remote);
    const state = draft.sync;
    if (theirs === undefined) {
      if (state.kind !== "synced") out.push({ ...draft, remote: undefined, sync: LOCAL, shared: null });
      continue;
    }
    const shared = theirs.share_token;
    const standing = standingOf(theirs);
    if (state.kind === "synced") {
      out.push(
        theirs.version === state.version
          ? { ...draft, shared, ...standing }
          : { ...draft, name: theirs.name, rules: theirs.rules, skin: theirs.skin, sync: { kind: "synced", version: theirs.version }, shared, ...standing },
      );
    } else if (state.kind === "conflict") {
      out.push({ ...draft, sync: { ...state, current: theirs.version }, shared, ...standing });
    } else if (state.kind !== "local" && state.version !== null && theirs.version > state.version) {
      out.push({ ...draft, sync: { kind: "conflict", version: state.version, current: theirs.version }, shared, ...standing });
    } else {
      out.push({ ...draft, shared, ...standing });
    }
  }
  for (const theirs of remote) if (!had.has(theirs.id)) out.push(make(theirs));
  return out;
}

/** What stays in the browser once the account is gone: only the drafts that were never the account's. */
export function withoutAccountDrafts<D extends Syncable>(drafts: D[]): D[] {
  return drafts.filter((d) => d.remote === undefined);
}
