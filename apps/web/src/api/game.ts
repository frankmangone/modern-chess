import { requestJson } from "./http";
import type { CatalogEntry, GameDetail } from "./types";

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
};
