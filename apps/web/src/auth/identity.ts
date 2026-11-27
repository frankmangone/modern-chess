import { ApiError } from "../api/http";
import { socialApi } from "../api/social";
import { exchangeIfSignedIn, remember } from "./session";
import { forgetGuest, loadGuest } from "./token-store";

let creating: Promise<string> | null = null;

/** The token of whoever is playing: the remembered one while it is good; else, for someone
 *  signed in, a new one traded for the provider's; else a new guest. Two calls at once share one
 *  request, so a double click never makes two guests. */
export function ensureToken(): Promise<string> {
  const stored = loadGuest();
  if (stored !== null) return Promise.resolve(stored.token);

  creating ??= (async () => {
    const account = await exchangeIfSignedIn();
    if (account !== null) return account;
    const guest = await socialApi.createGuest();
    remember(guest, false);
    return guest.token;
  })().finally(() => {
    creating = null;
  });
  return creating;
}

/** Runs a call that needs a person. If the server refuses the token (say its key was rotated),
 *  a new one is made (the account signed in again, or a new guest) and the call is tried once more. */
export async function withGuest<T>(call: (token: string) => Promise<T>): Promise<T> {
  const token = await ensureToken();
  try {
    return await call(token);
  } catch (error) {
    if (!(error instanceof ApiError) || error.status !== 401) throw error;
    forgetGuest();
    return call(await ensureToken());
  }
}

/** Runs a call that needs an account, not just a person: keeping work on the server. If the server says
 *  the token is a guest's (`account_required`) while someone is signed in, the token is one made before
 *  tokens said so, so it is traded once for a new one and the call is tried again. A real guest has no
 *  session to trade, so nothing is forgotten and the refusal is the answer. */
export async function withAccount<T>(call: (token: string) => Promise<T>): Promise<T> {
  try {
    return await withGuest(call);
  } catch (error) {
    if (!(error instanceof ApiError) || error.status !== 403 || error.code !== "account_required") throw error;
    if (loadGuest()?.account !== true) throw error;
    const fresh = await exchangeIfSignedIn(true);
    if (fresh === null) throw error;
    return call(fresh);
  }
}
