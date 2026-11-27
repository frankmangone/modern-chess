/** An outside sign-in service (Clerk, or the fake one used in development). Nothing else in the
 *  web app knows which one it is: the app asks it for a token, gives that token to our own
 *  server once, and from then on uses our own token. */
export interface AuthProvider {
  /** The label of the provider, for messages: "clerk" or "fake". */
  readonly name: string;
  /** Loads the provider. Resolves once it knows whether someone is signed in. */
  start(): Promise<void>;
  /** Whether the provider has a live session right now. */
  hasSession(): boolean;
  /** A fresh token of the live session, or null when there is none. Tokens last about a
   *  minute, so ask each time rather than keeping one. */
  getToken(): Promise<string | null>;
  /** Shows the provider's own sign-in. It does not wait: `subscribe` says when it worked. */
  openSignIn(): void;
  /** The same for someone new: it opens on making an account. */
  openSignUp(): void;
  signOut(): Promise<void>;
  /** Deletes the person at the provider too, after our own server has deleted the account. */
  deleteUser(): Promise<void>;
  /** Calls the listener whenever a session starts or ends. Returns a way to stop listening. */
  subscribe(listener: () => void): () => void;
}
