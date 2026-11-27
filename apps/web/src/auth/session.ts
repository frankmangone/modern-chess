import { createSignal } from "solid-js";
import { ApiError } from "../api/http";
import { socialApi } from "../api/social";
import type { Guest } from "../api/types";
import { type AuthProvider, authError } from "./provider";
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
const [ready, setReady] = createSignal(false);
const [google, setGoogle] = createSignal(false);
const [hint, setHint] = createSignal<string | undefined>(undefined);

/** What the page can show: whether sign-in is set up, the handle of the signed-in account,
 *  whether that account still has its set-up to go through, and whether a sign-in or sign-out is
 *  in progress (the trade for our token takes a moment, and the page should say so), and what the
 *  sign-in screen may offer (Google, a provider-specific line). `ready` is false until the provider
 *  has loaded (or been found absent), so a page can tell "not yet" from "not set up". */
export function useAuth() {
  return { ready, available, account, setupPending, working, google, hint };
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
 *  so its matches stay. With `fresh`, an account token we already hold is not good enough: the
 *  trade is made again (for a token that has since learned to say it is an account's). Null when
 *  nobody is signed in at the provider, or the trade failed (the person then plays on as a guest).
 *  Calls at once share one trade. */
export function exchangeIfSignedIn(fresh = false): Promise<string | null> {
  if (provider === null || !provider.hasSession()) return Promise.resolve(null);
  const current = provider;
  // Cleared by `.finally` on the promise, not inside it: a trade that needs no await (a stored
  // account token) would otherwise clear this before it was assigned and leave it stuck.
  exchanging ??= (async () => {
    setWorking("in");
    try {
      const stored = loadGuest();
      if (!fresh && stored?.account === true) {
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
  setGoogle(next?.google === true);
  setHint(next?.hint);
  showStored();
  if (next === null) {
    setReady(true);
    return;
  }
  setReady(false);
  await next.start();
  setReady(true);
  stopListening = next.subscribe(sessionChanged);
  if (next.hasSession()) await exchangeIfSignedIn();
  else if (account() !== null) sessionChanged();
}

function needProvider(): AuthProvider {
  if (provider === null) throw authError("unavailable");
  return provider;
}

/** Emails a sign-in code. The same for someone new and someone who is back. */
export function sendCode(email: string): Promise<void> {
  return needProvider().sendCode(email.trim());
}

/** Checks the code. When it is right the provider's session starts and the trade for our token
 *  follows by itself (see `sessionChanged`). */
export async function verifyCode(code: string): Promise<void> {
  await needProvider().verifyCode(code.trim());
  await finishSignIn();
}

/** Trades the provider's new session for our token, now, so the screen knows when it is done. */
async function finishSignIn(): Promise<void> {
  if ((await exchangeIfSignedIn()) === null) throw authError("unknown");
}

export function continueWithGoogle(returnTo: string): Promise<void> {
  return needProvider().continueWithGoogle(returnTo);
}

/** Finishes a trip to Google, and answers the path to land on. */
export async function completeRedirect(): Promise<string> {
  const path = await needProvider().completeRedirect();
  await finishSignIn();
  return path;
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
