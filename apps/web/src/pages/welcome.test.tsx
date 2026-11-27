import { Route, Router } from "@solidjs/router";
import { fireEvent, render, screen, waitFor, within } from "@solidjs/testing-library";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AuthProvider } from "../auth/provider";
import { configureAuth, useAuth } from "../auth/session";
import { saveGuest } from "../auth/token-store";
import { json, stubFetch } from "../test/server";
import { WelcomePage } from "./welcome";

function token(label: string): string {
  return `x.${btoa(JSON.stringify({ exp: Math.floor(Date.now() / 1000) + 600, label }))}.y`;
}

function provider(over: Partial<AuthProvider> = {}): AuthProvider {
  return {
    name: "test",
    start: () => Promise.resolve(),
    hasSession: () => true,
    getToken: () => Promise.resolve("t"),
    openSignIn: vi.fn(),
    openSignUp: vi.fn(),
    signOut: () => Promise.resolve(),
    deleteUser: () => Promise.resolve(),
    subscribe: () => () => undefined,
    ...over,
  };
}

function show() {
  render(() => (
    <Router>
      <Route path="/" component={() => <p>home</p>} />
      <Route path="/welcome" component={WelcomePage} />
    </Router>
  ));
  window.history.replaceState({}, "", "/welcome");
}

async function signedInWithSetup(pending = true) {
  saveGuest({ user: { id: "u1", handle: "Player-1234", setup_pending: pending }, token: token("start") }, true);
  await configureAuth(provider());
}

const available = { available: true, problem: null };

beforeEach(async () => {
  localStorage.clear();
  window.history.replaceState({}, "", "/welcome");
  await configureAuth(null);
});

describe("Welcome page", () => {
  it("asks someone who is not signed in to sign up", async () => {
    const p = provider({ hasSession: () => false });
    await configureAuth(p);
    show();

    // The header offers it too: this is the one on the page itself.
    fireEvent.click(within(screen.getByRole("main")).getByRole("button", { name: "Sign up" }));

    expect(p.openSignUp).toHaveBeenCalledOnce();
  });

  it("says there is nothing to do when the set-up is finished", async () => {
    await signedInWithSetup(false);
    show();

    expect(screen.getByText("There is nothing left to set up.")).toBeTruthy();
  });

  it("starts on the username, with the steps shown and no way on until a name is free", async () => {
    await signedInWithSetup();
    show();

    expect(screen.getByRole("list", { name: "Account set-up" })).toBeTruthy();
    expect((screen.getByRole("button", { name: "Continue" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("tells the person why a name cannot be used, and keeps Continue off", async () => {
    await signedInWithSetup();
    stubFetch({
      "GET /api/social/handles/Admin": () =>
        json({ available: false, problem: { code: "reserved", message: "That name is kept for us. Please choose another." } }),
    });
    show();

    fireEvent.input(screen.getByLabelText(/The name other players see/), { target: { value: "Admin" } });

    expect(await screen.findByText("That name is kept for us. Please choose another.")).toBeTruthy();
    expect((screen.getByRole("button", { name: "Continue" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("asks once typing pauses, not on every key", async () => {
    await signedInWithSetup();
    const calls = stubFetch({ "GET /api/social/handles/Grace": () => json(available) });
    show();
    const input = screen.getByLabelText(/The name other players see/);

    for (const text of ["G", "Gr", "Gra", "Grac", "Grace"]) fireEvent.input(input, { target: { value: text } });

    await screen.findByText("is free.", { exact: false });
    expect(calls.map((call) => call.route)).toEqual(["GET /api/social/handles/Grace"]);
  });

  it("chooses the name, moves on to the last step, and starting to play ends the set-up", async () => {
    await signedInWithSetup();
    const calls = stubFetch({
      "GET /api/social/handles/Grace": () => json(available),
      "PUT /api/social/me/handle": () =>
        json({ user: { id: "u1", handle: "Grace", setup_pending: true }, token: token("grace") }),
      "POST /api/social/me/setup": () =>
        json({ user: { id: "u1", handle: "Grace", setup_pending: false }, token: token("done") }),
    });
    show();

    fireEvent.input(screen.getByLabelText(/The name other players see/), { target: { value: "Grace" } });
    await screen.findByText("is free.", { exact: false });
    fireEvent.click(screen.getByRole("button", { name: "Continue" }));

    expect(await screen.findByText("You are in, Grace")).toBeTruthy();
    expect(useAuth().setupPending()).toBe(true);
    fireEvent.click(screen.getByRole("button", { name: "Start playing" }));

    expect(await screen.findByText("home")).toBeTruthy();
    expect(useAuth().setupPending()).toBe(false);
    expect(calls.map((call) => call.route)).toEqual([
      "GET /api/social/handles/Grace",
      "PUT /api/social/me/handle",
      "POST /api/social/me/setup",
    ]);
  });

  it("shows the server's reason when the name is taken in the meantime, and stays on the step", async () => {
    await signedInWithSetup();
    stubFetch({
      "GET /api/social/handles/Grace": () => json(available),
      "PUT /api/social/me/handle": () => json({ error: "That name is taken. Please choose another.", code: "taken" }, 409),
    });
    show();

    fireEvent.input(screen.getByLabelText(/The name other players see/), { target: { value: "Grace" } });
    await screen.findByText("is free.", { exact: false });
    fireEvent.click(screen.getByRole("button", { name: "Continue" }));

    expect(await screen.findByRole("alert")).toHaveProperty("textContent", "That name is taken. Please choose another.");
    await waitFor(() => expect((screen.getByRole("button", { name: "Continue" }) as HTMLButtonElement).disabled).toBe(false));
    expect(screen.queryByText("You are in, Grace")).toBeNull();
  });

  it("says it is getting the account ready while the sign-in finishes, not asking to sign up", async () => {
    let release: (value: Response) => void = () => undefined;
    vi.stubGlobal("fetch", vi.fn(() => new Promise<Response>((resolve) => (release = resolve))));
    const configuring = configureAuth(provider());
    show();

    expect(await screen.findByText("Getting your account ready…")).toBeTruthy();
    expect(within(screen.getByRole("main")).queryByRole("button", { name: "Sign up" })).toBeNull();

    // Let the trade end, so it does not hold up the next test.
    vi.spyOn(console, "warn").mockImplementation(() => undefined);
    release(new Response("{}", { status: 500 }));
    await configuring;
    vi.unstubAllGlobals();
  });

  it("says it is saving while the name is being chosen", async () => {
    await signedInWithSetup();
    stubFetch({
      "GET /api/social/handles/Grace": () => json(available),
      "PUT /api/social/me/handle": () => new Promise<Response>(() => undefined),
    });
    show();

    fireEvent.input(screen.getByLabelText(/The name other players see/), { target: { value: "Grace" } });
    await screen.findByText("is free.", { exact: false });
    fireEvent.click(screen.getByRole("button", { name: "Continue" }));

    expect(await screen.findByRole("button", { name: "Saving…" })).toHaveProperty("disabled", true);
  });
});
