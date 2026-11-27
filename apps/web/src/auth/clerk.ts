import type { AuthProvider } from "./provider";

/** Clerk, through `@clerk/clerk-js`, which is loaded only when sign-in is configured so that
 *  nobody else downloads it. */
export function clerkProvider(publishableKey: string): AuthProvider {
  type ClerkInstance = InstanceType<typeof import("@clerk/clerk-js").Clerk>;
  let clerk: ClerkInstance | null = null;
  const listeners = new Set<() => void>();

  return {
    name: "clerk",
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
    openSignIn() {
      clerk?.openSignIn({});
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
