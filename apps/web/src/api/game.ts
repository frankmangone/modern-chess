import { requestJson } from "./http";
import type {
  CatalogEntry,
  Command,
  DraftRecord,
  Filed,
  FiledGame,
  GameDetail,
  InstanceState,
  Listing,
  MyRecord,
  ReviewGame,
  ReviewLine,
  SharedGame,
} from "./types";

export const gameApi = {
  getCatalog: () => requestJson<CatalogEntry[]>("/api/game/catalog"),
  getGame: (slug: string) =>
    requestJson<GameDetail>(`/api/game/catalog/${encodeURIComponent(slug)}`),
  /** The caller's own record on a game. */
  getMyRecord: (slug: string, token: string) =>
    requestJson<MyRecord>(`/api/game/catalog/${encodeURIComponent(slug)}/record`, { token }),
  /** Opens a lobby in a variant (not a game: they share a slug only when a game has one variant). */
  createLobby: (variant: string, token: string) =>
    requestJson<{ id: string }>(`/api/game/catalog/${encodeURIComponent(variant)}/lobbies`, {
      method: "POST",
      token,
    }),
  join: (id: string, seat: string, token: string) =>
    requestJson<unknown>(`/api/game/instances/${encodeURIComponent(id)}/join`, {
      method: "POST",
      token,
      body: { seat },
    }),
  /** The match, or null when nothing changed since revision `since`. A token adds `you`. */
  getInstance: async (id: string, since: number | undefined, token?: string) => {
    const query = since === undefined ? "" : `?since=${since}`;
    const answer = await requestJson<InstanceState | { changed: false }>(
      `/api/game/instances/${encodeURIComponent(id)}${query}`,
      { token },
    );
    return "changed" in answer ? null : answer;
  },
  sendCommand: (id: string, revision: number, kind: Command, token: string) =>
    requestJson<InstanceState>(`/api/game/instances/${encodeURIComponent(id)}/commands`, {
      method: "POST",
      token,
      body: { revision, kind },
    }),
  // ---- Drafts: the games a person is writing, kept on their account ----
  listDrafts: (token: string) => requestJson<DraftRecord[]>("/api/game/drafts", { token }),
  getDraft: (id: string, token: string) =>
    requestJson<DraftRecord>(`/api/game/drafts/${encodeURIComponent(id)}`, { token }),
  createDraft: (draft: { name: string; rules: string; skin: string }, token: string) =>
    requestJson<DraftRecord>("/api/game/drafts", { method: "POST", token, body: draft }),
  /** Saves over `version`, the one the caller last saw. A 409 means someone saved since. */
  saveDraft: (id: string, draft: { name: string; rules: string; skin: string; version: number }, token: string) =>
    requestJson<DraftRecord>(`/api/game/drafts/${encodeURIComponent(id)}`, { method: "PUT", token, body: draft }),
  deleteDraft: (id: string, token: string) =>
    requestJson<void>(`/api/game/drafts/${encodeURIComponent(id)}`, { method: "DELETE", token }),
  /** The draft's private link secret: made the first time, the same after. */
  shareDraft: (id: string, token: string) =>
    requestJson<{ token: string }>(`/api/game/drafts/${encodeURIComponent(id)}/share`, { method: "PUT", token }),
  unshareDraft: (id: string, token: string) =>
    requestJson<void>(`/api/game/drafts/${encodeURIComponent(id)}/share`, { method: "DELETE", token }),
  // ---- Publishing: handing a draft in, and changing a game that is on the hub ----
  /** Hands in the draft the author saw at `version`: published at once, or waiting for review. */
  publishDraft: (id: string, version: number, listing: Listing, token: string) =>
    requestJson<Filed>(`/api/game/drafts/${encodeURIComponent(id)}/publish`, {
      method: "POST",
      token,
      body: { version, ...listing },
    }),
  /** Takes a game back out of review: it is a draft again. */
  withdrawDraft: (id: string, token: string) =>
    requestJson<void>(`/api/game/drafts/${encodeURIComponent(id)}/withdraw`, { method: "POST", token }),
  /** Starts a new revision of a published game: the draft to write it in. */
  reviseGame: (id: string, token: string) =>
    requestJson<DraftRecord>(`/api/game/games/${encodeURIComponent(id)}/revise`, { method: "POST", token }),
  /** The author's games that are no longer drafts. */
  listMyGames: (token: string) => requestJson<FiledGame[]>("/api/game/my-games", { token }),
  // ---- Review: for the people who look at a new author's first game ----
  reviewQueue: (token: string) => requestJson<ReviewLine[]>("/api/game/review", { token }),
  reviewGame: (id: string, token: string) =>
    requestJson<ReviewGame>(`/api/game/review/${encodeURIComponent(id)}`, { token }),
  approve: (id: string, token: string) =>
    requestJson<{ slug: string }>(`/api/game/review/${encodeURIComponent(id)}/approve`, { method: "POST", token }),
  reject: (id: string, reason: string, token: string) =>
    requestJson<void>(`/api/game/review/${encodeURIComponent(id)}/reject`, { method: "POST", token, body: { reason } }),
  /** What a private link opens. No sign-in needed. */
  getShared: (secret: string) => requestJson<SharedGame>(`/api/game/shared/${encodeURIComponent(secret)}`),
};
