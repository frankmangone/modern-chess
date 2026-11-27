import { authError, type AuthProvider } from "./provider";

// The fake provider for local development. It stands in for Clerk with a small sign-in service
// that the dev server runs (`dev-idp.ts`), and the social API is pointed at it like it would be
// at Clerk. It exists so sign-in can be built and tried with no outside account. It is only
// ever loaded by the dev server's build (`config.ts` checks `import.meta.env.DEV`).

const KEY = "modern-chess.fake-idp";
const PATH = "/dev-idp/token";
/** The code that always works here, so no mailbox is needed. */
export const FAKE_CODE = "424242";

function remembered(): string | null {
  try {
    return localStorage.getItem(KEY);
  } catch {
    return null;
  }
}

export function fakeProvider(): AuthProvider {
  const listeners = new Set<() => void>();
  let pending: string | null = null;
  const changed = () => {
    for (const listener of listeners) listener();
  };

  return {
    name: "fake",
    google: false,
    hint: `Development sign-in: any email works, and the code is ${FAKE_CODE}. The same email is the same account.`,
    start: () => Promise.resolve(),
    hasSession: () => remembered() !== null,
    async getToken() {
      const email = remembered();
      if (email === null) return null;
      const response = await fetch(PATH, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email }),
      });
      if (!response.ok) return null;
      const { token } = (await response.json()) as { token: string };
      return token;
    },
    sendCode(email) {
      if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) return Promise.reject(authError("invalid_email"));
      pending = email;
      return Promise.resolve();
    },
    verifyCode(code) {
      if (pending === null) return Promise.reject(authError("unknown"));
      if (code !== FAKE_CODE) return Promise.reject(authError("invalid_code"));
      try {
        localStorage.setItem(KEY, pending);
      } catch {
        return Promise.reject(authError("unknown"));
      }
      pending = null;
      changed();
      return Promise.resolve();
    },
    continueWithGoogle: () => Promise.reject(authError("unavailable")),
    completeRedirect: () => Promise.resolve("/"),
    signOut() {
      try {
        localStorage.removeItem(KEY);
      } catch {
        // Nothing to forget.
      }
      changed();
      return Promise.resolve();
    },
    deleteUser() {
      return this.signOut();
    },
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
  };
}
