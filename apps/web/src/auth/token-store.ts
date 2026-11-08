import type { Guest } from "../api/types";

const KEY = "modern-chess.guest";
/** A token this close to expiring is treated as expired, so it never dies mid-request. */
const MARGIN_SECONDS = 60;

export interface StoredGuest {
  token: string;
  handle: string;
}

/** The `exp` claim in seconds, or null when the token is not shaped like a JWT. The browser
 *  only uses it to decide when to ask for a new guest; the server is what really checks it. */
function expiry(token: string): number | null {
  const payload = token.split(".")[1];
  if (payload === undefined) return null;
  try {
    const claims: unknown = JSON.parse(atob(payload.replaceAll("-", "+").replaceAll("_", "/")));
    if (typeof claims === "object" && claims !== null && "exp" in claims) {
      const { exp } = claims as { exp: unknown };
      return typeof exp === "number" ? exp : null;
    }
  } catch {
    // Not base64 or not JSON: treat it as unusable below.
  }
  return null;
}

function isStored(value: unknown): value is StoredGuest {
  if (typeof value !== "object" || value === null) return false;
  const { token, handle } = value as Partial<StoredGuest>;
  return typeof token === "string" && typeof handle === "string";
}

/** The remembered guest, or null when there is none or its token has (nearly) expired. */
export function loadGuest(nowMs: number = Date.now()): StoredGuest | null {
  try {
    const raw = localStorage.getItem(KEY);
    if (raw === null) return null;
    const stored: unknown = JSON.parse(raw);
    if (!isStored(stored)) return null;
    const exp = expiry(stored.token);
    if (exp === null || exp - MARGIN_SECONDS <= nowMs / 1000) return null;
    return stored;
  } catch {
    return null;
  }
}

/** Remembers a guest. Storage can be blocked (private windows); the guest then lasts one visit. */
export function saveGuest(guest: Guest): void {
  try {
    const stored: StoredGuest = { token: guest.token, handle: guest.user.handle };
    localStorage.setItem(KEY, JSON.stringify(stored));
  } catch {
    // Not remembered; the next visit makes a new guest.
  }
}

export function forgetGuest(): void {
  try {
    localStorage.removeItem(KEY);
  } catch {
    // Nothing to forget.
  }
}
