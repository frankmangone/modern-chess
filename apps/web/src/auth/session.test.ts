import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ensureToken, withAccount, withGuest } from "./identity";
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
  deleted: number;
}

function fakeProvider(signedIn: boolean): Fake {
  const listeners = new Set<() => void>();
  const provider: Fake = {
    name: "test",
    google: true,
    hint: "A hint.",
    session: signedIn,
    deleted: 0,
    start: () => Promise.resolve(),
    hasSession: () => provider.session,
    getToken: () => Promise.resolve(provider.session ? "provider-token" : null),
    sendCode: () => Promise.resolve(),
    verifyCode: () => {
      provider.session = true;
      provider.emit();
      return Promise.resolve();
    },
    continueWithGoogle: () => Promise.resolve(),
    completeRedirect: () => Promise.resolve("/"),
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

describe("work in progress", () => {
  it("says a sign-in is in progress while our token is being traded for, and not after", async () => {
    let release: (value: Response) => void = () => undefined;
    stubFetch({
      "POST /api/social/session": () => new Promise<Response>((resolve) => (release = resolve)),
    });
    const p = fakeProvider(true);

    const configuring = configureAuth(p);
    await vi.waitFor(() => expect(useAuth().working()).toBe("in"));
    release(json(account("Ada")));
    await configuring;

    expect(useAuth().working()).toBeNull();
    expect(useAuth().account()).toBe("Ada");
  });

  it("clears it when the trade fails", async () => {
    stubFetch({ "POST /api/social/session": () => json({ error: "no" }, 401) });
    vi.spyOn(console, "warn").mockImplementation(() => undefined);

    await configureAuth(fakeProvider(true));

    expect(useAuth().working()).toBeNull();
  });

  it("says a sign-out is in progress while the provider ends the session", async () => {
    saveGuest(account("Ada"), true);
    let finish: () => void = () => undefined;
    const p = fakeProvider(true);
    p.signOut = () => new Promise<void>((resolve) => (finish = resolve));
    await configureAuth(p);

    const leaving = signOut();
    expect(useAuth().working()).toBe("out");
    finish();
    await leaving;

    expect(useAuth().working()).toBeNull();
  });

  it("trades again for a new sign-in after one that needed no trade", async () => {
    // Signed in already (a stored account token): nothing to trade, and nothing left in flight.
    saveGuest(account("Ada"), true);
    const p = fakeProvider(true);
    await configureAuth(p);
    await signOut();

    // Signing in again, in the same page, must reach the server for a token.
    const calls = stubFetch({ "POST /api/social/session": () => json(account("Grace")) });
    p.session = true;
    p.emit();
    await vi.waitFor(() => expect(useAuth().account()).toBe("Grace"));

    expect(calls).toHaveLength(1);
  });
});

describe("calls that need an account", () => {
  const refusal = () => new ApiError(403, "sign in with an account to do that", undefined, "account_required");

  it("trades once for a new token when the server says ours is a guest's, and tries again", async () => {
    saveGuest({ user: { id: "u1", handle: "Ada" }, token: token("made-before-the-claim", 600) }, true);
    const fresh = account("Ada");
    const calls = stubFetch({ "POST /api/social/session": () => json(fresh) });
    await configureAuth(fakeProvider(true));
    const attempts: string[] = [];

    const result = await withAccount(async (used) => {
      attempts.push(used);
      if (attempts.length === 1) throw refusal();
      return "kept";
    });

    expect(result).toBe("kept");
    expect(attempts[1]).toBe(fresh.token);
    expect(calls.filter((c) => c.route === "POST /api/social/session")).toHaveLength(1);
  });

  it("gives up after one trade when the new token is refused too", async () => {
    saveGuest({ user: { id: "u1", handle: "Ada" }, token: token("old", 600) }, true);
    stubFetch({ "POST /api/social/session": () => json(account("Ada")) });
    await configureAuth(fakeProvider(true));
    let attempts = 0;

    const failure = await withAccount(async () => {
      attempts += 1;
      throw refusal();
    }).catch((e: unknown) => e);

    expect(attempts).toBe(2);
    expect(failure).toBeInstanceOf(ApiError);
  });

  it("leaves a guest as they are: nothing is traded and nothing is forgotten", async () => {
    saveGuest(guest, false);
    const calls = stubFetch({ "POST /api/social/session": () => json(account("Ada")) });
    await configureAuth(fakeProvider(false));

    const failure = await withAccount(async () => {
      throw refusal();
    }).catch((e: unknown) => e);

    expect((failure as ApiError).code).toBe("account_required");
    expect(calls).toHaveLength(0);
    expect(loadGuest()?.token).toBe(guest.token);
  });

  it("keeps the token it has when the session is gone and the trade cannot be made", async () => {
    const old = token("old", 600);
    saveGuest({ user: { id: "u1", handle: "Ada" }, token: old }, true);
    stubFetch({});
    const provider = fakeProvider(true);
    await configureAuth(provider);
    provider.hasSession = () => false;

    const failure = await withAccount(async () => {
      throw refusal();
    }).catch((e: unknown) => e);

    expect((failure as ApiError).code).toBe("account_required");
    expect(loadGuest()?.token).toBe(old);
  });

  it("does not trade for another kind of refusal", async () => {
    saveGuest({ user: { id: "u1", handle: "Ada" }, token: token("t", 600) }, true);
    const calls = stubFetch({ "POST /api/social/session": () => json(account("Ada")) });
    await configureAuth(fakeProvider(true));

    const failure = await withAccount(async () => {
      throw new ApiError(403, "no", undefined, "something_else");
    }).catch((e: unknown) => e);

    expect((failure as ApiError).code).toBe("something_else");
    expect(calls).toHaveLength(0);
  });
});
