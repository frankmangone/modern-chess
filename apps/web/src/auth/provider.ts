/** Why a step of signing in failed, in the terms the page needs: which message to show. Anything
 *  a provider cannot place is "unknown". */
export type AuthErrorCode = "invalid_email" | "invalid_code" | "expired_code" | "too_many_tries" | "unavailable" | "unknown";

/** A failure the person can read: `message` is safe to show as it is. */
export class AuthError extends Error {
  readonly code: AuthErrorCode;

  constructor(code: AuthErrorCode, message: string) {
    super(message);
    this.name = "AuthError";
    this.code = code;
  }
}

/** The words for each failure, so every provider says the same thing. */
export const AUTH_MESSAGES: Record<AuthErrorCode, string> = {
  invalid_email: "That does not look like an email address.",
  invalid_code: "That code is not right. Check it and try again.",
  expired_code: "That code has expired. Send a new one.",
  too_many_tries: "Too many tries. Wait a minute, then try again.",
  unavailable: "Signing in is not available right now. Please try again later.",
  unknown: "That did not work. Please try again.",
};

export function authError(code: AuthErrorCode): AuthError {
  return new AuthError(code, AUTH_MESSAGES[code]);
}

/** An outside sign-in service (Clerk, or the fake one used in development). Nothing else in the
 *  web app knows which one it is: the app asks it for a token, gives that token to our own
 *  server once, and from then on uses our own token. The screens are ours; the provider only
 *  does what they ask: send a code, check it, go to Google and come back. */
export interface AuthProvider {
  /** The label of the provider, for messages: "clerk" or "fake". */
  readonly name: string;
  /** Whether "Continue with Google" works here. */
  readonly google: boolean;
  /** A line for the sign-in screen that only makes sense for this provider (the fake one says
   *  which code works). */
  readonly hint?: string;
  /** Loads the provider. Resolves once it knows whether someone is signed in. */
  start(): Promise<void>;
  /** Whether the provider has a live session right now. */
  hasSession(): boolean;
  /** A fresh token of the live session, or null when there is none. Tokens last about a
   *  minute, so ask each time rather than keeping one. */
  getToken(): Promise<string | null>;
  /** Emails a code to the address. It is the same for someone new and someone who is back: the
   *  provider works out which. Rejects with an `AuthError`. */
  sendCode(email: string): Promise<void>;
  /** Checks the code from the last `sendCode`. When it is right the session starts, and
   *  `subscribe` says so. Rejects with an `AuthError`. */
  verifyCode(code: string): Promise<void>;
  /** Leaves for Google. `returnTo` is the path to land on afterwards. It does not come back to
   *  the caller: the page is replaced, and `completeRedirect` runs on the page Google returns to. */
  continueWithGoogle(returnTo: string): Promise<void>;
  /** Finishes a trip to Google, on the page it returns to, and says where to go next. */
  completeRedirect(): Promise<string>;
  signOut(): Promise<void>;
  /** Deletes the person at the provider too, after our own server has deleted the account. */
  deleteUser(): Promise<void>;
  /** Calls the listener whenever a session starts or ends. Returns a way to stop listening. */
  subscribe(listener: () => void): () => void;
}
