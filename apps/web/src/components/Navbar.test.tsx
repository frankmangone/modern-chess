import { Router, Route } from "@solidjs/router";
import { fireEvent, render, screen } from "@solidjs/testing-library";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AuthProvider } from "../auth/provider";
import { configureAuth } from "../auth/session";
import { saveGuest } from "../auth/token-store";
import Navbar from "./Navbar";

function provider(over: Partial<AuthProvider> = {}): AuthProvider {
  return {
    name: "test",
    start: () => Promise.resolve(),
    hasSession: () => false,
    getToken: () => Promise.resolve(null),
    openSignIn: vi.fn(),
    signOut: vi.fn(() => Promise.resolve()),
    deleteUser: () => Promise.resolve(),
    subscribe: () => () => undefined,
    ...over,
  };
}

function show() {
  render(() => (
    <Router>
      <Route path="/" component={() => <Navbar />} />
    </Router>
  ));
}

beforeEach(async () => {
  await configureAuth(null);
});

describe("Navbar sign-in", () => {
  it("offers nothing when sign-in is not set up", () => {
    show();

    expect(screen.queryByRole("button", { name: "Sign in" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Sign out" })).toBeNull();
  });

  it("offers to sign in, which opens the provider's own sign-in", async () => {
    const p = provider();
    await configureAuth(p);
    show();

    fireEvent.click(screen.getByRole("button", { name: "Sign in" }));

    expect(p.openSignIn).toHaveBeenCalledOnce();
  });

  it("shows the account's handle, linking to the account page, and signs out", async () => {
    saveGuest({ user: { id: "u1", handle: "Ada" }, token: `x.${btoa(JSON.stringify({ exp: Date.now() / 1000 + 600 }))}.y` }, true);
    const p = provider({ hasSession: () => true, getToken: () => Promise.resolve("t") });
    vi.stubGlobal("fetch", vi.fn());
    await configureAuth(p);
    show();

    expect(screen.getByRole("link", { name: "Ada" }).getAttribute("href")).toBe("/account");
    fireEvent.click(screen.getByRole("button", { name: "Sign out" }));

    expect(p.signOut).toHaveBeenCalledOnce();
    expect(await screen.findByRole("button", { name: "Sign in" })).toBeTruthy();
    vi.unstubAllGlobals();
  });
});
