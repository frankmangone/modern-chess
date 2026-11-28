import { afterEach, describe, expect, it, vi } from "vitest";
import { ApiError, NetworkError, requestJson } from "./http";
import { gameApi } from "./game";
import { socialApi } from "./social";

function answer(status: number, body: unknown): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("api client", () => {
  it("a save that lost to another one says which version the server now holds", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(answer(409, { error: "This draft was changed somewhere else.", current_version: 7 })),
    );

    const failure = await requestJson("/api/game/drafts/x", { method: "PUT", body: {} }).catch((e: unknown) => e);

    expect(failure).toBeInstanceOf(ApiError);
    expect((failure as ApiError).status).toBe(409);
    expect((failure as ApiError).currentVersion).toBe(7);
  });

  it("an error carries the short code the server gave it", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(answer(403, { error: "sign in with an account to do that", code: "account_required" })),
    );

    const failure = await requestJson("/api/game/drafts", { token: "t" }).catch((e: unknown) => e);

    expect((failure as ApiError).code).toBe("account_required");
    expect((failure as ApiError).message).toBe("sign in with an account to do that");
  });

  it("an error with no version in it has none", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(answer(404, { error: "draft not found" })));

    const failure = await requestJson("/api/game/drafts/x").catch((e: unknown) => e);

    expect((failure as ApiError).currentVersion).toBeUndefined();
  });

  it("an empty 204 answer is a success with nothing in it", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(null, { status: 204 })));

    await expect(socialApi.deleteAccount("provider-token")).resolves.toBeUndefined();
    expect(fetch).toHaveBeenCalledWith("/api/social/me", expect.objectContaining({ method: "DELETE" }));
  });

  it("an account's token is asked for with the provider's token, handing over the guest", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(answer(200, { user: { id: "u", handle: "Ada" }, token: "ours" })));

    await socialApi.createSession("provider-token", "guest-token");

    const [path, init] = (fetch as ReturnType<typeof vi.fn>).mock.calls[0] as [string, RequestInit];
    expect(path).toBe("/api/social/session");
    expect((init.headers as Record<string, string>).Authorization).toBe("Bearer provider-token");
    expect(init.body).toBe(JSON.stringify({ guest_token: "guest-token" }));
  });

  it("api_client_throws_a_typed_error_for_non_2xx", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(answer(404, { error: "game not found" })));

    const failure = await gameApi.getGame("nope").catch((error: unknown) => error);

    expect(failure).toBeInstanceOf(ApiError);
    expect((failure as ApiError).status).toBe(404);
    expect((failure as ApiError).message).toBe("game not found");
  });

  it("keeps a readable message when the error body is not json", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response("<html>bad gateway</html>", { status: 502 })));

    const failure = await requestJson("/api/game/catalog").catch((error: unknown) => error);

    expect(failure).toBeInstanceOf(ApiError);
    expect((failure as ApiError).status).toBe(502);
    expect((failure as ApiError).message).toBe("the server could not answer (502)");
  });

  it("a_success_that_is_not_json_is_an_api_error_not_a_bare_syntax_error", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response("<html>sign in</html>", { status: 200 })));

    const failure = await requestJson("/api/game/catalog").catch((error: unknown) => error);

    expect(failure).toBeInstanceOf(ApiError);
    expect((failure as ApiError).message).toBe("the server answered with something unreadable");
  });

  it("throws a network error when the server cannot be reached", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new TypeError("Failed to fetch")));

    const failure = await gameApi.getCatalog().catch((error: unknown) => error);

    expect(failure).toBeInstanceOf(NetworkError);
  });

  it("asks the catalogue endpoint and returns its entries", async () => {
    const entries = [{ slug: "chess", name: "Chess" }];
    const fetchMock = vi.fn().mockResolvedValue(answer(200, entries));
    vi.stubGlobal("fetch", fetchMock);

    expect(await gameApi.getCatalog()).toEqual(entries);
    expect(fetchMock.mock.calls[0][0]).toBe("/api/game/catalog");
  });

  it("encodes the slug of a game", async () => {
    const fetchMock = vi.fn().mockResolvedValue(answer(200, {}));
    vi.stubGlobal("fetch", fetchMock);

    await gameApi.getGame("a/b c");

    expect(fetchMock.mock.calls[0][0]).toBe("/api/game/catalog/a%2Fb%20c");
  });

  it("creates a guest with a post", async () => {
    const guest = { user: { id: "u1", handle: "Guest-1234" }, token: "t" };
    const fetchMock = vi.fn().mockResolvedValue(answer(201, guest));
    vi.stubGlobal("fetch", fetchMock);

    expect(await socialApi.createGuest()).toEqual(guest);
    expect(fetchMock.mock.calls[0][0]).toBe("/api/social/guests");
    expect(fetchMock.mock.calls[0][1]).toMatchObject({ method: "POST" });
  });
});
