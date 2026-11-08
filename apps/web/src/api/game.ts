import { requestJson } from "./http";
import type { CatalogEntry, Command, GameDetail, InstanceState } from "./types";

export const gameApi = {
  getCatalog: () => requestJson<CatalogEntry[]>("/api/game/catalog"),
  getGame: (slug: string) =>
    requestJson<GameDetail>(`/api/game/catalog/${encodeURIComponent(slug)}`),
  createLobby: (slug: string, token: string) =>
    requestJson<{ id: string }>(`/api/game/catalog/${encodeURIComponent(slug)}/lobbies`, {
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
