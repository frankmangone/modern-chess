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

function storedSetup(): boolean {
  const stored = loadGuest();
  return stored?.account === true && stored.setup === true;
}

const [account, setAccount] = createSignal<string | null>(storedAccount());
const [setupPending, setSetupPending] = createSignal(storedSetup());
const [available, setAvailable] = createSignal(false);
const [working, setWorking] = createSignal<"in" | "out" | null>(null);

/** What the page can show: whether sign-in is set up, the handle of the signed-in account,
 *  whether that account still has its set-up to go through, and whether a sign-in or sign-out is
 *  in progress (the trade for our token takes a moment, and the page should say so). */
export function useAuth() {
  return { available, account, setupPending, working };
}

/** Shows what the token store holds. */
function showStored(): void {
  setAccount(storedAccount());
  setSetupPending(storedSetup());
}

function showSignedOut(): void {
  setAccount(null);
  setSetupPending(false);
}

/** Remembers a person that the server has just vouched for. */
export function remember(person: Guest, isAccount: boolean): void {
  saveGuest(person, isAccount);
  setAccount(isAccount ? person.user.handle : null);
  setSetupPending(isAccount && person.user.setup_pending === true);
}

/** The token of the live provider session traded for one of ours, adopting the current guest
 *  so its matches stay. Null when nobody is signed in at the provider, or the trade failed (the
 *  person then plays on as a guest). Calls at once share one trade. */
export function exchangeIfSignedIn(): Promise<string | null> {
  if (provider === null || !provider.hasSession()) return Promise.resolve(null);
  const current = provider;
  // Cleared by `.finally` on the promise, not inside it: a trade that needs no await (a stored
  // account token) would otherwise clear this before it was assigned and leave it stuck.
  exchanging ??= (async () => {
    setWorking("in");
    try {
      const stored = loadGuest();
      if (stored?.account === true) {
        showStored();
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
      setWorking(null);
    }
  })().finally(() => {
    exchanging = null;
  });
  return exchanging;
}

function sessionChanged(): void {
  if (provider?.hasSession()) {
    void exchangeIfSignedIn();
  } else if (account() !== null) {
    // The session ended (here or in another tab): this is a guest again.
    forgetGuest();
    showSignedOut();
  }
}

/** Sets the provider up (null for none: guests only) and picks up a session it already has. */
export async function configureAuth(next: AuthProvider | null): Promise<void> {
  stopListening?.();
  stopListening = null;
  provider = next;
  setAvailable(next !== null);
  showStored();
  if (next === null) return;
  await next.start();
  stopListening = next.subscribe(sessionChanged);
  if (next.hasSession()) await exchangeIfSignedIn();
  else if (account() !== null) sessionChanged();
}

export function signIn(): void {
  provider?.openSignIn();
}

export function signUp(): void {
  provider?.openSignUp();
}

export async function signOut(): Promise<void> {
  setWorking("out");
  try {
    await provider?.signOut();
    forgetGuest();
    showSignedOut();
  } finally {
    setWorking(null);
  }
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
  showSignedOut();
}
