import type { AuthProvider } from "./provider";

// The fake provider for local development. It stands in for Clerk with a small sign-in service
// that the dev server runs (`dev-idp.ts`), and the social API is pointed at it like it would be
// at Clerk. It exists so sign-in can be built and tried with no outside account. It is only
// ever loaded by the dev server's build (`config.ts` checks `import.meta.env.DEV`).

const KEY = "modern-chess.fake-idp";
const PATH = "/dev-idp/token";

function remembered(): string | null {
  try {
    return localStorage.getItem(KEY);
  } catch {
    return null;
  }
}

export function fakeProvider(): AuthProvider {
  const listeners = new Set<() => void>();
  const changed = () => {
    for (const listener of listeners) listener();
  };
  const askForEmail = () => {
    const email = window.prompt("Fake sign-in (development only). Any email works:", "ada@example.com")?.trim();
    if (!email) return;
    try {
      localStorage.setItem(KEY, email);
    } catch {
      return;
    }
    changed();
  };

  return {
    name: "fake",
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
    // Signing up and signing in are the same here: a new email is a new account.
    openSignIn: askForEmail,
    openSignUp: askForEmail,
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
