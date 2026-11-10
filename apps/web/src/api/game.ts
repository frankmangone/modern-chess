import { requestJson } from "./http";
import type { CatalogEntry, Command, GameDetail, InstanceState } from "./types";

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
};
