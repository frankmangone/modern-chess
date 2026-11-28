import { MemoryRouter, Route, createMemoryHistory } from "@solidjs/router";
import { fireEvent, render, screen, waitFor, within } from "@solidjs/testing-library";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AuthProvider } from "../auth/provider";
import { configureAuth } from "../auth/session";
import { forgetGuest, saveGuest } from "../auth/token-store";
import type { DraftRecord, FiledGame } from "../api/types";
import { json, stubFetch } from "../test/server";
import { BuilderPage } from "./builder";

vi.mock("../components/board-loader", () => ({ loadBoardElement: vi.fn().mockResolvedValue(undefined) }));
const engine = vi.hoisted(() => ({ checkSpec: vi.fn() }));
vi.mock("../builder/engine", () => engine);

function token(): string {
  return `x.${btoa(JSON.stringify({ exp: Math.floor(Date.now() / 1000) + 600 }))}.y`;
}

function provider(): AuthProvider {
  return {
    name: "test",
    start: () => Promise.resolve(),
    hasSession: () => true,
    getToken: () => Promise.resolve("t"),
    google: false,
    sendCode: () => Promise.resolve(),
    verifyCode: () => Promise.resolve(),
    continueWithGoogle: () => Promise.resolve(),
    completeRedirect: () => Promise.resolve("/"),
    signOut: () => Promise.resolve(),
    deleteUser: () => Promise.resolve(),
    subscribe: () => () => undefined,
  };
}

async function signedIn() {
  saveGuest({ user: { id: "u1", handle: "Ada", setup_pending: false }, token: token() }, true);
  await configureAuth(provider());
}

function record(over: Partial<DraftRecord> = {}): DraftRecord {
  return {
    id: "r1",
    name: "Stride",
    rules: JSON.stringify({ name: "STRIDE" }),
    skin: "",
    version: 1,
    status: "draft",
    slug: null,
    listing: null,
    revises: null,
    rejection: null,
    share_token: null,
    created_at: "2026-11-27T10:00:00Z",
    updated_at: "2026-11-27T10:00:00Z",
    ...over,
  };
}

function filed(over: Partial<FiledGame> = {}): FiledGame {
  return {
    id: "g1",
    name: "First",
    status: "published",
    slug: "first",
    revisions: 1,
    working_copy: null,
    updated_at: "2026-11-27T10:00:00Z",
    ...over,
  };
}

function show() {
  const history = createMemoryHistory();
  history.set({ value: "/build" });
  render(() => (
    <MemoryRouter history={history}>
      <Route path="/build" component={BuilderPage} />
    </MemoryRouter>
  ));
}

const catalogue = { "GET /api/game/catalog": () => json([]) };
const SLOW = { timeout: 4000 };

beforeEach(async () => {
  localStorage.clear();
  forgetGuest();
  await configureAuth(null);
  engine.checkSpec.mockReset();
  engine.checkSpec.mockImplementation((text: string) => {
    try {
      return Promise.resolve({ ok: true, name: (JSON.parse(text) as { name?: string }).name ?? "?", players: ["A", "B"] });
    } catch {
      return Promise.resolve({ ok: false, message: "expected value", line: 1, column: 1, path: null });
    }
  });
});

/** Waits until the engine has read the draft and the account has it, then opens the form. */
async function openForm() {
  const button = (await screen.findByRole("button", { name: "Publish…" })) as HTMLButtonElement;
  await waitFor(() => expect(button.disabled).toBe(false));
  fireEvent.click(button);
  return screen.findByRole("form", { name: "Publish this game" });
}

/** Opens the form on the draft on show, and fills it in as a person would. */
async function fillIn(tagline = "Two in a row", description = "A game for two.") {
  const form = await openForm();
  fireEvent.input(within(form).getByLabelText("Tagline"), { target: { value: tagline } });
  fireEvent.input(within(form).getByLabelText("Description"), { target: { value: description } });
  fireEvent.click(within(form).getByRole("checkbox"));
  return form;
}

describe("Publishing from the workbench", () => {
  it("offers to publish only a draft that is on the account and that the engine reads", async () => {
    await signedIn();
    stubFetch({ ...catalogue, "GET /api/game/drafts": () => json([record()]) });
    show();

    const button = (await screen.findByRole("button", { name: "Publish…" })) as HTMLButtonElement;
    await waitFor(() => expect(button.disabled).toBe(false));

    fireEvent.input(screen.getByLabelText("The game's rules, as JSON"), { target: { value: "{" } });
    await waitFor(() => expect((screen.getByRole("button", { name: "Publish…" }) as HTMLButtonElement).disabled).toBe(true));
    expect(screen.getByText(/Waiting for your latest changes|The engine has to read the rules first/)).toBeTruthy();
  });

  it("hands the draft in with the version it saw, and says it is waiting for review", async () => {
    await signedIn();
    const calls = stubFetch({
      ...catalogue,
      "GET /api/game/drafts": () => json([record({ version: 4 })]),
      "POST /api/game/drafts/r1/publish": () => json({ status: "in_review" }, 202),
      "GET /api/game/my-games": () => json([filed({ status: "in_review", slug: null, revisions: 0 })]),
    });
    show();
    const form = await fillIn();
    expect(within(form).getByText(/your first game, so we look at it/)).toBeTruthy();
    expect(within(form).getByRole("button", { name: "Hand in for review" })).toBeTruthy();

    fireEvent.click(within(form).getByRole("button", { name: "Hand in for review" }));

    expect(await screen.findByText(/was handed in\. We look at the first game/)).toBeTruthy();
    expect(calls.find((c) => c.route === "POST /api/game/drafts/r1/publish")?.body).toEqual({
      version: 4,
      tagline: "Two in a row",
      description: "A game for two.",
      licence: "CC-BY-SA-4.0",
    });
    // It is no longer a draft, so it is gone from the workbench's drafts and is in Your games.
    await waitFor(() => expect(screen.queryByRole("form", { name: "Publish this game" })).toBeNull());
    expect(await screen.findByText("Waiting to be looked at")).toBeTruthy();
  });

  it("says the game is on the hub when it was published at once, and publishes with no review wording for a trusted author", async () => {
    await signedIn();
    stubFetch({
      ...catalogue,
      "GET /api/game/drafts": () => json([record()]),
      "GET /api/game/my-games": () => json([filed()]),
      "POST /api/game/drafts/r1/publish": () => json({ status: "published", slug: "stride" }),
    });
    show();
    const form = await fillIn();
    expect(within(form).getByText(/It goes on the hub as soon as you publish it/)).toBeTruthy();

    fireEvent.click(within(form).getByRole("button", { name: "Publish" }));

    expect(await screen.findByText("“Stride” is on the hub.")).toBeTruthy();
  });

  it("keeps the form and its words when the server refuses, and says why", async () => {
    await signedIn();
    stubFetch({
      ...catalogue,
      "GET /api/game/drafts": () => json([record()]),
      "POST /api/game/drafts/r1/publish": () => json({ error: "These rules cannot be published: too many players." }, 422),
    });
    show();
    const form = await fillIn();

    fireEvent.click(within(form).getByRole("button", { name: "Hand in for review" }));

    expect(await screen.findByText("These rules cannot be published: too many players.")).toBeTruthy();
    expect((within(form).getByLabelText("Tagline") as HTMLInputElement).value).toBe("Two in a row");
    expect(screen.getByRole("option", { name: "Stride" })).toBeTruthy();
  });

  it("does not send a listing with nothing written or without the licence accepted", async () => {
    await signedIn();
    const calls = stubFetch({ ...catalogue, "GET /api/game/drafts": () => json([record()]) });
    show();
    const form = await openForm();

    fireEvent.click(within(form).getByRole("button", { name: "Hand in for review" }));
    expect(await within(form).findByText("Say what the game is in a line.")).toBeTruthy();

    fireEvent.input(within(form).getByLabelText("Tagline"), { target: { value: "Two in a row" } });
    fireEvent.input(within(form).getByLabelText("Description"), { target: { value: "A game for two." } });
    fireEvent.click(within(form).getByRole("button", { name: "Hand in for review" }));
    expect(await within(form).findByText(/Agree to share the game under CC BY-SA 4.0/)).toBeTruthy();
    expect(calls.some((c) => c.route.endsWith("/publish"))).toBe(false);
  });

  it("can be cancelled", async () => {
    await signedIn();
    stubFetch({ ...catalogue, "GET /api/game/drafts": () => json([record()]) });
    show();
    const form = await fillIn();

    fireEvent.click(within(form).getByRole("button", { name: "Cancel" }));

    expect(screen.queryByRole("form", { name: "Publish this game" })).toBeNull();
  });
});

describe("Your games", () => {
  it("lists what is waiting and what is on the hub, with a way to the page", async () => {
    await signedIn();
    stubFetch({
      ...catalogue,
      "GET /api/game/drafts": () => json([record()]),
      "GET /api/game/my-games": () =>
        json([
          filed({ id: "a", name: "Live", revisions: 2 }),
          filed({ id: "b", name: "Waiting", status: "in_review", slug: null, revisions: 0 }),
        ]),
    });
    show();

    const panel = await screen.findByRole("region", { name: "Your games on the hub" });
    expect(within(panel).getByText("On the hub, revision 2")).toBeTruthy();
    expect(within(panel).getByText("Waiting to be looked at")).toBeTruthy();
    expect(within(panel).getByRole("link", { name: "View on the hub" }).getAttribute("href")).toBe("/games/first");
  });

  it("takes a game back out of review, and it is a draft again with the reason it was sent back", async () => {
    await signedIn();
    let back = false;
    const calls = stubFetch({
      ...catalogue,
      "GET /api/game/drafts": () => json(back ? [record({ id: "g1", name: "First", rejection: "Say what it is" })] : []),
      "GET /api/game/my-games": () => json(back ? [] : [filed({ status: "in_review", slug: null, revisions: 0 })]),
      "POST /api/game/drafts/g1/withdraw": () => {
        back = true;
        return new Response(null, { status: 204 });
      },
    });
    show();

    fireEvent.click(await screen.findByRole("button", { name: "Take it back" }));

    await waitFor(() => expect(calls.some((c) => c.route === "POST /api/game/drafts/g1/withdraw")).toBe(true));
    expect(await screen.findByText("Say what it is", undefined, SLOW)).toBeTruthy();
    expect(screen.getByText("Sent back.")).toBeTruthy();
  });

  it("starts a new revision of a game on the hub and shows its working copy", async () => {
    await signedIn();
    let started = false;
    const calls = stubFetch({
      ...catalogue,
      "GET /api/game/drafts": () =>
        json(started ? [record({ id: "w1", name: "First", revises: "g1", listing: { tagline: "t", description: "d", licence: "CC-BY-SA-4.0" } })] : []),
      "GET /api/game/my-games": () => json([filed({ working_copy: started ? "w1" : null })]),
      "POST /api/game/games/g1/revise": () => {
        started = true;
        return json(record({ id: "w1", name: "First", revises: "g1" }), 201);
      },
    });
    show();

    fireEvent.click(await screen.findByRole("button", { name: "Edit" }));

    await waitFor(() => expect(calls.some((c) => c.route === "POST /api/game/games/g1/revise")).toBe(true));
    expect(await screen.findByText(/A new revision of/, undefined, SLOW)).toBeTruthy();
    expect(await screen.findByRole("button", { name: "Continue editing" })).toBeTruthy();
  });
});
