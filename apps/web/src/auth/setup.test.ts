import { beforeEach, describe, expect, it } from "vitest";
import { ApiError } from "../api/http";
import { json, stubFetch } from "../test/server";
import { chooseHandle, checkHandle, finishSetup } from "./setup";
import { configureAuth, useAuth } from "./session";
import { loadGuest, saveGuest } from "./token-store";

function token(label: string): string {
  const payload = btoa(JSON.stringify({ exp: Math.floor(Date.now() / 1000) + 600, label }));
  return `x.${payload}.y`;
}

const pending = { user: { id: "u1", handle: "Player-1234", setup_pending: true }, token: token("pending") };

beforeEach(async () => {
  localStorage.clear();
  await configureAuth(null);
  saveGuest(pending, true);
  await configureAuth(null);
});

describe("account set-up state", () => {
  it("is pending for a stored account that has not finished, and not for a guest", async () => {
    expect(useAuth().setupPending()).toBe(true);

    saveGuest({ user: { id: "g", handle: "Guest-1", setup_pending: true }, token: token("guest") }, false);
    await configureAuth(null);

    expect(useAuth().setupPending()).toBe(false);
  });
});

describe("choosing a username", () => {
  it("sends the name with our token and remembers the new token and name", async () => {
    const fresh = { user: { id: "u1", handle: "Ada", setup_pending: true }, token: token("ada") };
    const calls = stubFetch({ "PUT /api/social/me/handle": () => json(fresh) });

    await chooseHandle("Ada");

    expect(calls).toEqual([
      { route: "PUT /api/social/me/handle", query: "", authorization: `Bearer ${pending.token}`, body: { handle: "Ada" } },
    ]);
    expect(loadGuest()).toMatchObject({ handle: "Ada", token: fresh.token, account: true, setup: true });
    expect(useAuth().account()).toBe("Ada");
    expect(useAuth().setupPending()).toBe(true);
  });

  it("passes on the server's words when the name is refused, and changes nothing", async () => {
    stubFetch({
      "PUT /api/social/me/handle": () => json({ error: "That name is taken. Please choose another.", code: "taken" }, 409),
    });

    const failure = await chooseHandle("Ada").catch((error: unknown) => error);

    expect(failure).toBeInstanceOf(ApiError);
    expect((failure as ApiError).message).toBe("That name is taken. Please choose another.");
    expect(useAuth().account()).toBe("Player-1234");
  });
});

describe("checking a username", () => {
  it("asks about the name as written, escaped for the address", async () => {
    const calls = stubFetch({
      "GET /api/social/handles/a%2Fb": () =>
        json({ available: false, problem: { code: "bad_characters", message: "Use only letters." } }),
    });

    const answer = await checkHandle("a/b");

    expect(answer.problem?.code).toBe("bad_characters");
    expect(calls[0]?.authorization).toBe(`Bearer ${pending.token}`);
  });
});

describe("finishing the set-up", () => {
  it("clears the pending state here and in the store", async () => {
    stubFetch({
      "POST /api/social/me/setup": () =>
        json({ user: { id: "u1", handle: "Ada", setup_pending: false }, token: token("done") }),
    });

    await finishSetup();

    expect(useAuth().setupPending()).toBe(false);
    expect(loadGuest()).toMatchObject({ handle: "Ada", account: true, setup: false });
  });
});
