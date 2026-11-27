import { requestJson } from "./http";
import type { CatalogEntry, Command, DraftRecord, GameDetail, InstanceState, SharedGame } from "./types";

export const gameApi = {
  getCatalog: () => requestJson<CatalogEntry[]>("/api/game/catalog"),
  getGame: (slug: string) =>
    requestJson<GameDetail>(`/api/game/catalog/${encodeURIComponent(slug)}`),
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
  /** What a private link opens. No sign-in needed. */
  getShared: (secret: string) => requestJson<SharedGame>(`/api/game/shared/${encodeURIComponent(secret)}`),
};
