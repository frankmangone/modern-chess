import { ApiError } from "../api/http";
import { socialApi } from "../api/social";
import { forgetGuest, loadGuest, saveGuest } from "./token-store";

let creating: Promise<string> | null = null;

/** The token of the remembered guest, making a guest first when there is none. Two calls at
 *  once share one request, so a double click never makes two guests. */
export function ensureToken(): Promise<string> {
  const stored = loadGuest();
  if (stored !== null) return Promise.resolve(stored.token);

  creating ??= socialApi
    .createGuest()
    .then((guest) => {
      saveGuest(guest);
      return guest.token;
    })
    .finally(() => {
      creating = null;
    });
  return creating;
}

/** Runs a call that needs a guest. If the server refuses the token (say its key was rotated),
 *  the guest is replaced and the call is tried once more. */
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
