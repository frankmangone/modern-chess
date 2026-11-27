import { clerkAppearance } from "./clerk-appearance";
import type { AuthProvider } from "./provider";

/** Clerk, through `@clerk/clerk-js` and its sign-in screens in `@clerk/ui`, both loaded only
 *  when sign-in is configured so that nobody else downloads them. Without the UI package the
 *  screens never appear ("Clerk was not loaded with Ui components"). */
/** Where Clerk wants to take the person after a sign-in or sign-up. Its default is a full page
 *  load, even to the page they are already on, which throws the page away, reloads Clerk and
 *  leaves the header blank for a moment. Staying put is what we want; anywhere else is a real
 *  navigation (the set-up page is reached by our own router, as soon as the account exists). */
function navigate(to: string): Promise<void> {
  const target = new URL(to, window.location.href);
  if (target.href !== window.location.href) window.location.assign(target.href);
  return Promise.resolve();
}

export function clerkProvider(publishableKey: string): AuthProvider {
  type ClerkInstance = InstanceType<typeof import("@clerk/clerk-js").Clerk>;
  let clerk: ClerkInstance | null = null;
  const listeners = new Set<() => void>();

  return {
    name: "clerk",
    async start() {
      const [{ Clerk }, { ui }] = await Promise.all([import("@clerk/clerk-js"), import("@clerk/ui")]);
      const instance = new Clerk(publishableKey);
      await instance.load({
        ui: { ClerkUI: ui.ClerkUI },
        appearance: clerkAppearance,
        routerPush: navigate,
        routerReplace: navigate,
      });
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
    openSignUp() {
      clerk?.openSignUp({});
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
