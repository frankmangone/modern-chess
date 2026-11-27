import { AuthError, type AuthErrorCode, type AuthProvider, authError } from "./provider";

/** Where Google sends the person back to: a page of ours that finishes the trip. */
export const REDIRECT_PATH = "/sso-callback";

/** Clerk's error codes, as the screen's reasons. */
const CODES: Record<string, AuthErrorCode> = {
  form_param_format_invalid: "invalid_email",
  form_identifier_invalid: "invalid_email",
  form_code_incorrect: "invalid_code",
  form_param_value_invalid: "invalid_code",
  verification_failed: "invalid_code",
  verification_expired: "expired_code",
  too_many_requests: "too_many_tries",
  rate_limit_exceeded: "too_many_tries",
};

function errorCode(error: unknown): string | null {
  if (typeof error !== "object" || error === null || !("errors" in error)) return null;
  const { errors } = error as { errors: unknown };
  const first = Array.isArray(errors) ? (errors[0] as { code?: unknown } | undefined) : undefined;
  return typeof first?.code === "string" ? first.code : null;
}

/** The error as the screen should see it: ours stay as they are, Clerk's are put into words. */
function mapped(error: unknown): AuthError {
  if (error instanceof AuthError) return error;
  const code = errorCode(error);
  return authError((code !== null && CODES[code]) || "unknown");
}

/** Clerk, through `@clerk/clerk-js` alone: its sign-in and sign-up calls, with none of its screens
 *  (those are ours). Loaded only when sign-in is configured so that nobody else downloads it. */
export function clerkProvider(publishableKey: string): AuthProvider {
  type ClerkInstance = InstanceType<typeof import("@clerk/clerk-js").Clerk>;
  let clerk: ClerkInstance | null = null;
  // Which attempt the code is for: an existing person signs in, a new one signs up.
  let attempt: "sign-in" | "sign-up" | null = null;
  const listeners = new Set<() => void>();

  const loaded = (): ClerkInstance => {
    if (clerk === null) throw authError("unavailable");
    return clerk;
  };

  /** Clerk's sign-in and sign-up resources. */
  const client = () => {
    const resources = loaded().client;
    if (resources === undefined) throw authError("unavailable");
    return resources;
  };

  /** Makes the session of a finished attempt the live one. */
  async function finish(done: { status: string | null; createdSessionId: string | null; missingFields?: string[] }): Promise<void> {
    if (done.status !== "complete" || done.createdSessionId === null) {
      // Said out loud for whoever runs the Clerk instance: the usual cause is a setting that asks
      // for more than an email (a password, a name), which these screens do not collect.
      console.warn("Clerk did not finish the attempt:", done.status, done.missingFields ?? []);
      throw authError("unknown");
    }
    await loaded().setActive({ session: done.createdSessionId });
  }

  return {
    name: "clerk",
    google: true,
    async start() {
      const { Clerk } = await import("@clerk/clerk-js");
      const instance = new Clerk(publishableKey);
      await instance.load();
      clerk = instance;
      // The session is compared each time: Clerk also calls back for changes that do not
      // start or end one (the person's name, say).
      let signedIn = instance.session != null;
      instance.addListener(() => {
        const now = instance.session != null;
        if (now === signedIn) return;
        signedIn = now;
        for (const listener of listeners) listener();
      });
    },
    hasSession: () => clerk?.session != null,
    async getToken() {
      return (await clerk?.session?.getToken()) ?? null;
    },
    async sendCode(email) {
      const { signIn, signUp } = client();
      try {
        // Someone who has an account signs in...
        const started = await signIn.create({ identifier: email });
        const factor = started.supportedFirstFactors?.find((candidate) => candidate.strategy === "email_code");
        if (factor === undefined || !("emailAddressId" in factor)) throw authError("unavailable");
        await signIn.prepareFirstFactor({ strategy: "email_code", emailAddressId: factor.emailAddressId });
        attempt = "sign-in";
      } catch (error) {
        if (errorCode(error) !== "form_identifier_not_found") throw mapped(error);
        // ... and anyone else makes one.
        try {
          await signUp.create({ emailAddress: email });
          await signUp.prepareEmailAddressVerification({ strategy: "email_code" });
          attempt = "sign-up";
        } catch (signUpError) {
          throw mapped(signUpError);
        }
      }
    },
    async verifyCode(code) {
      const { signIn, signUp } = client();
      try {
        if (attempt === "sign-in") {
          await finish(await signIn.attemptFirstFactor({ strategy: "email_code", code }));
        } else if (attempt === "sign-up") {
          await finish(await signUp.attemptEmailAddressVerification({ code }));
        } else {
          throw authError("unknown");
        }
      } catch (error) {
        throw mapped(error);
      }
    },
    async continueWithGoogle(returnTo) {
      try {
        await client().signIn.authenticateWithRedirect({
          strategy: "oauth_google",
          redirectUrl: `${window.location.origin}${REDIRECT_PATH}`,
          redirectUrlComplete: `${window.location.origin}${returnTo}`,
        });
      } catch (error) {
        throw mapped(error);
      }
    },
    async completeRedirect() {
      let next = "/";
      try {
        // A person new to us is made an account on the way (Clerk transfers the attempt).
        await loaded().handleRedirectCallback(
          { signInFallbackRedirectUrl: "/", signUpFallbackRedirectUrl: "/" },
          (to) => {
            next = to;
            return Promise.resolve();
          },
        );
      } catch (error) {
        throw mapped(error);
      }
      const target = new URL(next, window.location.href);
      return target.origin === window.location.origin ? `${target.pathname}${target.search}` : "/";
    },
    async signOut() {
      await clerk?.signOut();
    },
    async deleteUser() {
      await clerk?.user?.delete();
    },
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
  };
}
