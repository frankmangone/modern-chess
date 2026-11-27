import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ensureToken, withGuest } from "./identity";
import type { AuthProvider } from "./provider";
import { configureAuth, deleteAccount, signOut, useAuth } from "./session";
import { loadGuest, saveGuest } from "./token-store";
import { json, stubFetch } from "../test/server";
import { ApiError } from "../api/http";

/** A token the browser reads the expiry of, like the real ones (the server is what checks it). */
function token(label: string, secondsFromNow: number): string {
  const payload = btoa(JSON.stringify({ exp: Math.floor(Date.now() / 1000) + secondsFromNow, label }));
  return `x.${payload}.y`;
}

interface Fake extends AuthProvider {
  session: boolean;
  emit(): void;
  opened: number;
  deleted: number;
}

function fakeProvider(signedIn: boolean): Fake {
  const listeners = new Set<() => void>();
  const provider: Fake = {
    name: "test",
    session: signedIn,
    opened: 0,
    deleted: 0,
    start: () => Promise.resolve(),
    hasSession: () => provider.session,
    getToken: () => Promise.resolve(provider.session ? "provider-token" : null),
    openSignIn: () => {
      provider.opened += 1;
    },
    signOut: () => {
      provider.session = false;
      provider.emit();
      return Promise.resolve();
    },
    deleteUser: () => {
      provider.deleted += 1;
      provider.session = false;
      return Promise.resolve();
    },
    subscribe: (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    emit: () => listeners.forEach((listener) => listener()),
  };
  return provider;
}

const account = (handle = "Guest-1111") => ({ user: { id: "u1", handle }, token: token(`account-${handle}`, 600) });
const guest = { user: { id: "g1", handle: "Guest-1111" }, token: token("guest", 600) };

beforeEach(async () => {
  await configureAuth(null);
});
afterEach(() => {
  vi.unstubAllGlobals();
});

describe("signing in", () => {
  it("trades the provider's token for ours, handing over the guest so its matches stay", async () => {
    saveGuest(guest);
    const calls = stubFetch({ "POST /api/social/session": () => json(account()) });
    const provider = fakeProvider(false);
    await configureAuth(provider);
    expect(useAuth().account()).toBeNull();

    provider.session = true;
    provider.emit();
    await vi.waitFor(() => expect(useAuth().account()).toBe("Guest-1111"));

    expect(calls).toHaveLength(1);
    expect(calls[0]).toMatchObject({ route: "POST /api/social/session", authorization: "Bearer provider-token", body: { guest_token: guest.token } });
    expect(loadGuest()).toMatchObject({ handle: "Guest-1111", account: true });
  });

  it("sends no guest when there is none", async () => {
    const calls = stubFetch({ "POST /api/social/session": () => json(account("Ada")) });
    const provider = fakeProvider(false);
    await configureAuth(provider);

    provider.session = true;
    provider.emit();
    await vi.waitFor(() => expect(useAuth().account()).toBe("Ada"));

    expect(calls[0]?.body).toBeUndefined();
  });

  it("picks up a session the provider already has when the page opens", async () => {
    stubFetch({ "POST /api/social/session": () => json(account("Ada")) });

    await configureAuth(fakeProvider(true));

    expect(useAuth().account()).toBe("Ada");
  });

  it("is a guest again, and stays playable, when the trade is refused", async () => {
    stubFetch({
      "POST /api/social/session": () => json({ error: "no" }, 401),
      "POST /api/social/guests": () => json(guest),
    });

    await configureAuth(fakeProvider(true));

    expect(useAuth().account()).toBeNull();
    expect(await ensureToken()).toBe(guest.token);
  });

  it("shows no sign-in when none is set up", async () => {
    await configureAuth(null);

    expect(useAuth().available()).toBe(false);
  });
});

describe("staying signed in", () => {
  it("trades again without asking when our token has expired but the provider's session lives", async () => {
    saveGuest({ user: { id: "u1", handle: "Ada" }, token: token("old", -10) }, true);
    const fresh = account("Ada");
    const calls = stubFetch({ "POST /api/social/session": () => json(fresh) });
    const provider = fakeProvider(true);
    provider.hasSession = () => false; // not yet known when the page opens: only ensureToken asks
    await configureAuth(provider);
    provider.hasSession = () => true;

    expect(await ensureToken()).toBe(fresh.token);
    expect(calls).toHaveLength(1);
  });

  it("two calls at once make one trade", async () => {
    const fresh = account("Ada");
    const calls = stubFetch({ "POST /api/social/session": () => json(fresh) });
    const provider = fakeProvider(true);
    provider.hasSession = () => false;
    await configureAuth(provider);
    provider.hasSession = () => true;

    await Promise.all([ensureToken(), ensureToken()]);

    expect(calls).toHaveLength(1);
  });

  it("trades again when the server refuses our token, rather than turning an account into a guest", async () => {
    saveGuest({ user: { id: "u1", handle: "Ada" }, token: token("revoked", 600) }, true);
    const fresh = account("Ada");
    stubFetch({ "POST /api/social/session": () => json(fresh) });
    const provider = fakeProvider(true);
    provider.hasSession = () => false;
    await configureAuth(provider);
    provider.hasSession = () => true;
    const attempts: string[] = [];

    const result = await withGuest(async (used) => {
      attempts.push(used);
      if (attempts.length === 1) throw new ApiError(401, "no");
      return "ok";
    });

    expect(result).toBe("ok");
    expect(attempts[1]).toBe(fresh.token);
  });
});

describe("signing out and deleting", () => {
  it("signs out at the provider and forgets our token, so the next visit is a new guest", async () => {
    stubFetch({ "POST /api/social/session": () => json(account("Ada")) });
    const provider = fakeProvider(true);
    await configureAuth(provider);

    await signOut();

    expect(provider.session).toBe(false);
    expect(useAuth().account()).toBeNull();
    expect(loadGuest()).toBeNull();
  });

  it("is signed out here when the session ends somewhere else", async () => {
    stubFetch({ "POST /api/social/session": () => json(account("Ada")) });
    const provider = fakeProvider(true);
    await configureAuth(provider);

    provider.session = false;
    provider.emit();

    expect(useAuth().account()).toBeNull();
    expect(loadGuest()).toBeNull();
  });

  it("deletes with the provider's token, then at the provider, then forgets", async () => {
    const calls = stubFetch({
      "POST /api/social/session": () => json(account("Ada")),
      "DELETE /api/social/me": () => new Response(null, { status: 204 }),
    });
    const provider = fakeProvider(true);
    await configureAuth(provider);

    await deleteAccount();

    expect(calls.at(-1)).toMatchObject({ route: "DELETE /api/social/me", authorization: "Bearer provider-token" });
    expect(provider.deleted).toBe(1);
    expect(useAuth().account()).toBeNull();
    expect(loadGuest()).toBeNull();
  });

  it("does not delete at the provider when our server refused", async () => {
    stubFetch({
      "POST /api/social/session": () => json(account("Ada")),
      "DELETE /api/social/me": () => json({ error: "no" }, 401),
    });
    const provider = fakeProvider(true);
    await configureAuth(provider);

    await expect(deleteAccount()).rejects.toBeInstanceOf(ApiError);

    expect(provider.deleted).toBe(0);
    expect(useAuth().account()).toBe("Ada");
  });
});
