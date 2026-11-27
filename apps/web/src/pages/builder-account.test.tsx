import { MemoryRouter, Route, createMemoryHistory } from "@solidjs/router";
import { fireEvent, render, screen, waitFor, within } from "@solidjs/testing-library";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AuthProvider } from "../auth/provider";
import { configureAuth } from "../auth/session";
import { forgetGuest, saveGuest } from "../auth/token-store";
import type { DraftRecord } from "../api/types";
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
    share_token: null,
    created_at: "2026-11-27T10:00:00Z",
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

const rulesBox = () => screen.getByLabelText("The game's rules, as JSON") as HTMLTextAreaElement;
const SLOW = { timeout: 4000 };
const catalogue = { "GET /api/game/catalog": () => json([]) };

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

describe("Workbench, signed out", () => {
  it("says the draft is only in this browser, and offers to sign in", async () => {
    stubFetch(catalogue);
    show();

    expect(await screen.findByText(/Only in this browser\./)).toBeTruthy();
    expect(screen.getByRole("link", { name: "Sign in" }).getAttribute("href")).toBe("/sign-in?next=%2Fbuild");
    expect(screen.queryByRole("button", { name: "Share a link" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Save to my account" })).toBeNull();
  });

  it("keeps the same text box while typing, so the cursor stays where it is", async () => {
    stubFetch(catalogue);
    show();
    const before = rulesBox();

    fireEvent.input(before, { target: { value: "{}" } });

    expect(rulesBox()).toBe(before);
  });
});

describe("Workbench, signed in", () => {
  it("shows the drafts on the account, and leaves out the starter nobody touched", async () => {
    await signedIn();
    stubFetch({ ...catalogue, "GET /api/game/drafts": () => json([record()]) });
    show();

    expect(await screen.findByText("Saved to your account")).toBeTruthy();
    const names = within(screen.getByLabelText("Draft")).getAllByRole("option").map((o) => o.textContent);
    expect(names).toEqual(["Stride"]);
  });

  it("asks the account with the person's own token", async () => {
    await signedIn();
    const calls = stubFetch({ ...catalogue, "GET /api/game/drafts": () => json([record()]) });
    show();
    await screen.findByText("Saved to your account");

    expect(calls.find((c) => c.route === "GET /api/game/drafts")?.authorization).toMatch(/^Bearer /);
  });

  it("puts a draft that is only here on the account when asked", async () => {
    await signedIn();
    const calls = stubFetch({
      ...catalogue,
      "GET /api/game/drafts": () => json([]),
      "POST /api/game/drafts": () => json(record({ id: "new", name: "March", version: 1 }), 201),
    });
    show();

    fireEvent.click(await screen.findByRole("button", { name: "Save to my account" }));

    expect(await screen.findByText("Saved to your account", undefined, SLOW)).toBeTruthy();
    const made = calls.find((c) => c.route === "POST /api/game/drafts");
    expect(made?.body).toMatchObject({ name: "March" });
  });

  it("saves a change over the version it started from, a moment after typing stops", async () => {
    await signedIn();
    const calls = stubFetch({
      ...catalogue,
      "GET /api/game/drafts": () => json([record({ version: 3 })]),
      "PUT /api/game/drafts/r1": () => json(record({ version: 4, rules: '{"name":"NEW"}' })),
    });
    show();
    await screen.findByText("Saved to your account");

    fireEvent.input(rulesBox(), { target: { value: '{"name":"NEW"}' } });
    fireEvent.input(rulesBox(), { target: { value: '{"name":"NEW"}' } });

    await waitFor(() => expect(calls.filter((c) => c.route === "PUT /api/game/drafts/r1")).toHaveLength(1), SLOW);
    expect(calls.find((c) => c.route === "PUT /api/game/drafts/r1")?.body).toMatchObject({
      rules: '{"name":"NEW"}',
      version: 3,
    });
    expect(await screen.findByText("Saved to your account")).toBeTruthy();
  });

  it("keeps the work and says so when the save cannot be made", async () => {
    await signedIn();
    stubFetch({
      ...catalogue,
      "GET /api/game/drafts": () => json([record()]),
      "PUT /api/game/drafts/r1": () => json({ error: "something went wrong" }, 500),
    });
    show();
    await screen.findByText("Saved to your account");

    fireEvent.input(rulesBox(), { target: { value: '{"name":"NEW"}' } });

    expect(await screen.findByText(/Not saved to your account yet/, undefined, SLOW)).toBeTruthy();
    expect(rulesBox().value).toBe('{"name":"NEW"}');
    await waitFor(() => expect(localStorage.getItem("hubbable.drafts")).toContain("NEW"));
  });

  it("lets the person choose when the draft was changed somewhere else too", async () => {
    await signedIn();
    const calls = stubFetch({
      ...catalogue,
      "GET /api/game/drafts": () => json([record({ version: 1 })]),
      "PUT /api/game/drafts/r1": () => json({ error: "This draft was changed somewhere else.", current_version: 2 }, 409),
      "GET /api/game/drafts/r1": () => json(record({ version: 2, rules: '{"name":"THEIRS"}' })),
    });
    show();
    await screen.findByText("Saved to your account");

    fireEvent.input(rulesBox(), { target: { value: '{"name":"MINE"}' } });

    expect(await screen.findByText(/changed somewhere else too/, undefined, SLOW)).toBeTruthy();
    expect(rulesBox().value).toBe('{"name":"MINE"}');

    fireEvent.click(screen.getByRole("button", { name: "Load the other one" }));

    await waitFor(() => expect(rulesBox().value).toBe('{"name":"THEIRS"}'));
    expect(await screen.findByText("Saved to your account")).toBeTruthy();
    expect(calls.filter((c) => c.route === "PUT /api/game/drafts/r1")).toHaveLength(1);
  });

  it("saves the person's version over the other one when they keep it", async () => {
    await signedIn();
    let first = true;
    const calls = stubFetch({
      ...catalogue,
      "GET /api/game/drafts": () => json([record({ version: 1 })]),
      "PUT /api/game/drafts/r1": () => {
        if (first) {
          first = false;
          return json({ error: "This draft was changed somewhere else.", current_version: 2 }, 409);
        }
        return json(record({ version: 3 }));
      },
    });
    show();
    await screen.findByText("Saved to your account");
    fireEvent.input(rulesBox(), { target: { value: '{"name":"MINE"}' } });
    await screen.findByText(/changed somewhere else too/, undefined, SLOW);

    fireEvent.click(screen.getByRole("button", { name: "Keep the one here" }));

    await waitFor(() => expect(calls.filter((c) => c.route === "PUT /api/game/drafts/r1")).toHaveLength(2), SLOW);
    expect(calls.filter((c) => c.route === "PUT /api/game/drafts/r1")[1].body).toMatchObject({ version: 2 });
  });

  it("shares a draft by a private link, shows it, and stops sharing", async () => {
    await signedIn();
    const calls = stubFetch({
      ...catalogue,
      "GET /api/game/drafts": () => json([record()]),
      "PUT /api/game/drafts/r1/share": () => json({ token: "SECRETSECRETSECRETSECRE" }),
      "DELETE /api/game/drafts/r1/share": () => new Response(null, { status: 204 }),
    });
    show();
    await screen.findByText("Saved to your account");

    fireEvent.click(screen.getByRole("button", { name: "Share a link" }));

    const field = (await screen.findByLabelText("Private link")) as HTMLInputElement;
    expect(field.value).toBe(`${window.location.origin}/build/shared/SECRETSECRETSECRETSECRE`);
    expect(screen.getByText(/Anyone with this link can play the game/)).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Stop sharing" }));

    await waitFor(() => expect(screen.queryByLabelText("Private link")).toBeNull());
    expect(calls.some((c) => c.route === "DELETE /api/game/drafts/r1/share")).toBe(true);
    expect(screen.getByRole("button", { name: "Share a link" })).toBeTruthy();
  });

  it("makes a new link by turning the old one off first", async () => {
    await signedIn();
    const order: string[] = [];
    stubFetch({
      ...catalogue,
      "GET /api/game/drafts": () => json([record({ share_token: "OLDOLDOLDOLDOLDOLDOLDO" })]),
      "DELETE /api/game/drafts/r1/share": () => {
        order.push("off");
        return new Response(null, { status: 204 });
      },
      "PUT /api/game/drafts/r1/share": () => {
        order.push("on");
        return json({ token: "NEWNEWNEWNEWNEWNEWNEWN" });
      },
    });
    show();
    await screen.findByLabelText("Private link");

    fireEvent.click(screen.getByRole("button", { name: "Make a new link" }));

    await waitFor(() => expect((screen.getByLabelText("Private link") as HTMLInputElement).value).toContain("NEWNEW"));
    expect(order).toEqual(["off", "on"]);
  });

  it("deletes the account's copy with the draft, and keeps the draft if that fails", async () => {
    await signedIn();
    let fail = true;
    const calls = stubFetch({
      ...catalogue,
      "GET /api/game/drafts": () => json([record(), record({ id: "r2", name: "Other" })]),
      "DELETE /api/game/drafts/r1": () => (fail ? json({ error: "something went wrong" }, 500) : new Response(null, { status: 204 })),
    });
    show();
    await screen.findByText("Saved to your account");

    fireEvent.click(screen.getByRole("button", { name: "Delete this draft" }));
    fireEvent.click(screen.getByRole("button", { name: "Yes, delete it" }));
    expect(await screen.findByText(/could not delete the draft from your account/)).toBeTruthy();
    expect(within(screen.getByLabelText("Draft")).getAllByRole("option")).toHaveLength(2);

    fail = false;
    fireEvent.click(screen.getByRole("button", { name: "Yes, delete it" }));

    await waitFor(() => expect(within(screen.getByLabelText("Draft")).getAllByRole("option")).toHaveLength(1));
    expect(calls.filter((c) => c.route === "DELETE /api/game/drafts/r1")).toHaveLength(2);
  });

  it("starts a new draft on the account", async () => {
    await signedIn();
    const calls = stubFetch({
      ...catalogue,
      "GET /api/game/drafts": () => json([record()]),
      "POST /api/game/drafts": () => json(record({ id: "r9", name: "March", version: 1 }), 201),
    });
    show();
    await screen.findByText("Saved to your account");

    fireEvent.click(screen.getByRole("button", { name: "New draft" }));
    fireEvent.click(screen.getByRole("button", { name: "Create draft" }));

    await waitFor(() => expect(calls.some((c) => c.route === "POST /api/game/drafts")).toBe(true), SLOW);
  });

  it("does not drop the browser's drafts when the account cannot be reached, and says so", async () => {
    await signedIn();
    stubFetch({ ...catalogue, "GET /api/game/drafts": () => json({ error: "something went wrong" }, 500) });
    show();

    expect(await screen.findByText(/could not reach your account to load your drafts/)).toBeTruthy();
    expect(rulesBox().value).toContain("MARCH");
  });

  it("takes the account's drafts out of the browser when the person signs out", async () => {
    await signedIn();
    stubFetch({ ...catalogue, "GET /api/game/drafts": () => json([record({ name: "Private" })]) });
    show();
    await screen.findByText("Saved to your account");
    await waitFor(() => expect(localStorage.getItem("hubbable.drafts")).toContain("Private"));

    forgetGuest();
    await configureAuth(null);

    await waitFor(() => expect(localStorage.getItem("hubbable.drafts")).not.toContain("Private"));
    expect(await screen.findByText(/Only in this browser\./)).toBeTruthy();
  });
});
