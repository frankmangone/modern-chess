import { requestJson } from "./http";
import type { Guest } from "./types";

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
  /** Deletes the account. It asks for the provider's token, not ours. */
  deleteAccount: (providerToken: string) =>
    requestJson<void>("/api/social/me", { method: "DELETE", token: providerToken }),
};
