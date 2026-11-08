import { requestJson } from "./http";
import type { Guest } from "./types";

export const socialApi = {
  createGuest: () => requestJson<Guest>("/api/social/guests", { method: "POST" }),
};
