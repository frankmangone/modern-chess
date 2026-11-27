import { requestJson } from "./http";
import type { Guest, HandleCheck } from "./types";

export const socialApi = {
  createGuest: () => requestJson<Guest>("/api/social/guests", { method: "POST" }),
  /** Trades a sign-in provider's token for one of ours. The guest's own token, when there is
   *  one, lets the account adopt that guest and keep its matches. */
  createSession: (providerToken: string, guestToken?: string) =>
    requestJson<Guest>("/api/social/session", {
      method: "POST",
      token: providerToken,
      body: guestToken === undefined ? undefined : { guest_token: guestToken },
    }),
  /** Whether a username can be chosen. */
  checkHandle: (token: string, name: string) =>
    requestJson<HandleCheck>(`/api/social/handles/${encodeURIComponent(name)}`, { token }),
  /** Chooses the username during the set-up. The answer carries a new token: the name is in it. */
  chooseHandle: (token: string, handle: string) =>
    requestJson<Guest>("/api/social/me/handle", { method: "PUT", token, body: { handle } }),
  /** Ends the set-up. */
  finishSetup: (token: string) => requestJson<Guest>("/api/social/me/setup", { method: "POST", token }),
  /** Deletes the account. It asks for the provider's token, not ours. */
  deleteAccount: (providerToken: string) =>
    requestJson<void>("/api/social/me", { method: "DELETE", token: providerToken }),
};
