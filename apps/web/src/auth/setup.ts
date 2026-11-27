import { socialApi } from "../api/social";
import type { HandleCheck } from "../api/types";
import { withGuest } from "./identity";
import { remember } from "./session";

// The calls the account set-up makes. Each answer that carries a person is remembered, so the
// header and the token store change with it: the username is in our token.

/** Whether a username can be chosen, and why not when it cannot. */
export function checkHandle(name: string): Promise<HandleCheck> {
  return withGuest((token) => socialApi.checkHandle(token, name));
}

/** Chooses the username. Rejects with an `ApiError` whose message says what is wrong (taken,
 *  not allowed) in words for the person. */
export async function chooseHandle(name: string): Promise<void> {
  const person = await withGuest((token) => socialApi.chooseHandle(token, name));
  remember(person, true);
}

/** Ends the set-up. */
export async function finishSetup(): Promise<void> {
  const person = await withGuest((token) => socialApi.finishSetup(token));
  remember(person, true);
}
