import { createSignal } from "solid-js";
import { ApiError } from "../api/http";
import { socialApi } from "../api/social";
import type { Guest } from "../api/types";
import type { AuthProvider } from "./provider";
import { forgetGuest, loadGuest, saveGuest } from "./token-store";

// Who is signed in. A person is either a guest or an account; both are a token of ours kept in
// the token store, and an account also has a live session at the sign-in provider. The provider
// is asked for its token only to get one of ours, and our token is what every call carries.

let provider: AuthProvider | null = null;
let stopListening: (() => void) | null = null;
let exchanging: Promise<string | null> | null = null;

function storedAccount(): string | null {
  const stored = loadGuest();
  return stored?.account === true ? stored.handle : null;
}

const [account, setAccount] = createSignal<string | null>(storedAccount());
const [available, setAvailable] = createSignal(false);

/** What the page can show: whether sign-in is set up, and the handle of the signed-in account. */
export function useAuth() {
  return { available, account };
}

/** Remembers a person that the server has just vouched for. */
export function remember(person: Guest, isAccount: boolean): void {
  saveGuest(person, isAccount);
  setAccount(isAccount ? person.user.handle : null);
}

/** The token of the live provider session traded for one of ours, adopting the current guest
 *  so its matches stay. Null when nobody is signed in at the provider, or the trade failed (the
 *  person then plays on as a guest). Calls at once share one trade. */
export function exchangeIfSignedIn(): Promise<string | null> {
  if (provider === null || !provider.hasSession()) return Promise.resolve(null);
  const current = provider;
  exchanging ??= (async () => {
    try {
      const stored = loadGuest();
      if (stored?.account === true) {
        setAccount(stored.handle);
        return stored.token;
      }
      const providerToken = await current.getToken();
      if (providerToken === null) return null;
      const person = await socialApi.createSession(providerToken, stored?.token);
      remember(person, true);
      return person.token;
    } catch (error) {
      // Said out loud: a refused trade leaves the person looking signed out, and a silent failure
      // here once hid a real bug (a token the server could not read).
      console.warn("Signing in failed:", error instanceof ApiError ? `${error.status} ${error.message}` : error);
      return null;
    } finally {
      exchanging = null;
    }
  })();
  return exchanging;
}

function sessionChanged(): void {
  if (provider?.hasSession()) {
    void exchangeIfSignedIn();
  } else if (account() !== null) {
    // The session ended (here or in another tab): this is a guest again.
    forgetGuest();
    setAccount(null);
  }
}

/** Sets the provider up (null for none: guests only) and picks up a session it already has. */
export async function configureAuth(next: AuthProvider | null): Promise<void> {
  stopListening?.();
  stopListening = null;
  provider = next;
  setAvailable(next !== null);
  setAccount(storedAccount());
  if (next === null) return;
  await next.start();
  stopListening = next.subscribe(sessionChanged);
  if (next.hasSession()) await exchangeIfSignedIn();
  else if (account() !== null) sessionChanged();
}

export function signIn(): void {
  provider?.openSignIn();
}

export async function signOut(): Promise<void> {
  await provider?.signOut();
  forgetGuest();
  setAccount(null);
}

/** Deletes the account at our server, then at the provider. The provider's token is the proof
 *  that someone is signed in right now. */
export async function deleteAccount(): Promise<void> {
  if (provider === null) throw new Error("sign-in is not set up");
  const providerToken = await provider.getToken();
  if (providerToken === null) throw new Error("you are not signed in");
  await socialApi.deleteAccount(providerToken);
  await provider.deleteUser();
  forgetGuest();
  setAccount(null);
}
