import type { AuthProvider } from "./provider";

/** The provider this build is set up for, or null when only guests can play. It is chosen by
 *  `VITE_AUTH_PROVIDER` (`clerk` or `fake`); `make dev` sets it, see `make dev AUTH=...`. The
 *  fake one exists only in the dev server: a production build never contains it. */
export async function createProvider(env: Record<string, unknown> = import.meta.env): Promise<AuthProvider | null> {
  const wanted = env.VITE_AUTH_PROVIDER;
  if (wanted === "fake" && import.meta.env.DEV) {
    const { fakeProvider } = await import("./fake");
    return fakeProvider();
  }
  if (wanted === "clerk") {
    const key = env.VITE_CLERK_PUBLISHABLE_KEY;
    if (typeof key !== "string" || key === "") {
      console.warn("VITE_AUTH_PROVIDER is clerk but VITE_CLERK_PUBLISHABLE_KEY is missing: guests only.");
      return null;
    }
    const { clerkProvider } = await import("./clerk");
    return clerkProvider(key);
  }
  return null;
}
