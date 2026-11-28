/** Handing a draft in to the hub, as a state machine, and the words the form uses.
 *
 *  `step` is the one place that knows which state follows which event; the page and the tests both
 *  read it. The server decides whether a game is published at once or waits for review, so nothing
 *  here guesses: `done` holds what it said. */

import type { Filed, Listing } from "../api/types";
import { type Sync } from "./sync";

/** The licence every published game is shared under, as the server names it. It is accepted by name,
 *  so a form that was opened before the licence changed cannot accept the new one by accident. */
export const LICENCE = "CC-BY-SA-4.0";
export const LICENCE_NAME = "CC BY-SA 4.0";
export const LICENCE_URL = "https://creativecommons.org/licenses/by-sa/4.0/";

/** The limits the server holds a listing to. */
export const MAX_TAGLINE = 140;
export const MAX_DESCRIPTION = 2000;

export type Publishing =
  /** Nothing is being published. */
  | { kind: "closed" }
  /** The form is open. */
  | { kind: "open" }
  /** The draft is on its way. */
  | { kind: "sending" }
  /** The server said no, in words. The form stays open, with what was written. */
  | { kind: "refused"; reason: string }
  /** The draft was handed in. `name` is what it was called then. */
  | { kind: "done"; filed: Filed; name: string };

export type PublishingEvent =
  | { type: "open" }
  | { type: "close" }
  | { type: "send" }
  | { type: "filed"; filed: Filed; name: string }
  | { type: "refused"; reason: string }
  /** The person has read what happened. */
  | { type: "dismiss" };

export const CLOSED: Publishing = { kind: "closed" };

/** The state after an event. An event that means nothing in a state leaves it as it is, so a late
 *  answer cannot reopen a form the person has closed. */
export function step(state: Publishing, event: PublishingEvent): Publishing {
  switch (event.type) {
    case "open":
      return state.kind === "closed" ? { kind: "open" } : state;
    case "close":
      return state.kind === "open" || state.kind === "refused" ? CLOSED : state;
    case "send":
      return state.kind === "open" || state.kind === "refused" ? { kind: "sending" } : state;
    case "filed":
      return state.kind === "sending" ? { kind: "done", filed: event.filed, name: event.name } : state;
    case "refused":
      return state.kind === "sending" ? { kind: "refused", reason: event.reason } : state;
    case "dismiss":
      return state.kind === "done" ? CLOSED : state;
  }
}

/** Whether the form is on show. */
export function formOpen(state: Publishing): boolean {
  return state.kind === "open" || state.kind === "sending" || state.kind === "refused";
}

/** What a draft needs before it can be handed in, or why it cannot be yet. */
export function whyNotYet(draft: { remote?: string; sync: Sync }, rulesOk: boolean, lookOk: boolean): string | null {
  if (draft.remote === undefined) return "Save it to your account first.";
  if (!rulesOk) return "The engine has to read the rules first.";
  if (!lookOk) return "The look has to be readable first.";
  if (draft.sync.kind !== "synced") {
    return draft.sync.kind === "conflict"
      ? "Choose which version to keep first."
      : "Waiting for your latest changes to be saved.";
  }
  return null;
}

/** What is wrong with the words of a listing, or nothing. The server checks the same limits. */
export function listingProblem(listing: Listing, accepted: boolean): string | null {
  const tagline = listing.tagline.trim();
  const description = listing.description.trim();
  if (tagline === "") return "Say what the game is in a line.";
  if (tagline.length > MAX_TAGLINE) return `The line is too long (at most ${MAX_TAGLINE} characters).`;
  if (description === "") return "Describe the game for someone who has never seen it.";
  if (description.length > MAX_DESCRIPTION) return `The description is too long (at most ${MAX_DESCRIPTION} characters).`;
  if (!accepted) return `Agree to share the game under ${LICENCE_NAME} to publish it.`;
  return null;
}

/** What a person is told when the draft has been handed in. */
export function describeDone(state: Extract<Publishing, { kind: "done" }>): string {
  return state.filed.status === "published"
    ? `“${state.name}” is on the hub.`
    : `“${state.name}” was handed in. We look at the first game of each new author before it appears, so it will be on the hub once it has been looked at. You can take it back until then.`;
}
