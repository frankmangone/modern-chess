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
