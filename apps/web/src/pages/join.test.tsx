import { Route, Router } from "@solidjs/router";
import { fireEvent, render, screen, waitFor, within } from "@solidjs/testing-library";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { authError, type AuthProvider } from "../auth/provider";
import { configureAuth, useAuth } from "../auth/session";
import { json, stubFetch } from "../test/server";
import { SignInPage, SignUpPage, safeNext } from "./join";

function token(label: string): string {
  return `x.${btoa(JSON.stringify({ exp: Math.floor(Date.now() / 1000) + 600, label }))}.y`;
}

interface Fake extends AuthProvider {
  session: boolean;
  sent: string[];
  checked: string[];
  google_: string[];
  emit(): void;
}

function provider(over: Partial<Fake> = {}): Fake {
  const listeners = new Set<() => void>();
  const p: Fake = {
    name: "test",
    google: true,
    session: false,
    sent: [],
    checked: [],
    google_: [],
    start: () => Promise.resolve(),
    hasSession: () => p.session,
    getToken: () => Promise.resolve(p.session ? "provider-token" : null),
    sendCode: (email) => {
      p.sent.push(email);
      return Promise.resolve();
    },
    verifyCode: (code) => {
      p.checked.push(code);
      p.session = true;
      p.emit();
      return Promise.resolve();
    },
    continueWithGoogle: (returnTo) => {
      p.google_.push(returnTo);
      return Promise.resolve();
    },
    completeRedirect: () => Promise.resolve("/"),
    signOut: () => Promise.resolve(),
    deleteUser: () => Promise.resolve(),
    subscribe: (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    emit: () => listeners.forEach((listener) => listener()),
    ...over,
  };
  return p;
}

function show(path = "/sign-in") {
  window.history.replaceState({}, "", path);
  render(() => (
    <Router>
      <Route path="/" component={() => <p>home</p>} />
      <Route path="/games/:slug" component={() => <p>a game</p>} />
      <Route path="/welcome" component={() => <p>welcome page</p>} />
      <Route path="/sign-in" component={SignInPage} />
      <Route path="/sign-up" component={SignUpPage} />
    </Router>
  ));
}

const session = (pending: boolean) => ({ user: { id: "u1", handle: "Ada", setup_pending: pending }, token: token("a") });

const typeEmail = (value: string) => fireEvent.input(screen.getByLabelText("Your email"), { target: { value } });
const typeCode = (value: string) => fireEvent.input(screen.getByLabelText("The code"), { target: { value } });
const asked = () => screen.getByRole("button", { name: "Email me a code" });

beforeEach(async () => {
  localStorage.clear();
  await configureAuth(null);
});

describe("safeNext", () => {
  it("keeps a path of ours, and sends anything that could leave the site home", () => {
    expect(safeNext("/games/chess")).toBe("/games/chess");
    expect(safeNext("/games/chess?tab=rules")).toBe("/games/chess?tab=rules");
    for (const bad of [undefined, "", "games", "//evil.example", "https://evil.example", "/\\evil.example", ["//x", "/ok"]]) {
      expect(safeNext(bad as string)).toBe("/");
    }
    expect(safeNext(["/ok", "/other"])).toBe("/ok");
  });
});

describe("Sign-in page", () => {
  it("waits for the provider before saying anything about it", () => {
    void configureAuth(provider({ start: () => new Promise<void>(() => undefined) }));
    show();

    expect(screen.getByText("One moment…")).toBeTruthy();
  });

  it("says so, kindly, when sign-in is not set up here", async () => {
    await configureAuth(null);
    show();

    expect(await screen.findByText(/not set up here/)).toBeTruthy();
    expect(screen.queryByLabelText("Your email")).toBeNull();
  });

  it("offers Google only when the provider can", async () => {
    await configureAuth(provider({ google: false }));
    show();
    await screen.findByLabelText("Your email");
    expect(screen.queryByRole("button", { name: "Continue with Google" })).toBeNull();
  });

  it("sends a code to the email, then asks for it", async () => {
    const p = provider();
    await configureAuth(p);
    show();

    typeEmail(" ada@example.com ");
    fireEvent.click(asked());

    expect(await screen.findByLabelText("The code")).toBeTruthy();
    expect(p.sent).toEqual(["ada@example.com"]);
    expect(screen.getByText("ada@example.com")).toBeTruthy();
  });

  it("says why when the email is refused, and stays on the email", async () => {
    await configureAuth(provider({ sendCode: () => Promise.reject(authError("invalid_email")) }));
    show();

    typeEmail("nope");
    fireEvent.click(asked());

    expect((await screen.findByRole("alert")).textContent).toBe("That does not look like an email address.");
    expect(screen.queryByLabelText("The code")).toBeNull();
  });

  it("checks the code as soon as the sixth digit is in, trades for our token and goes home", async () => {
    const p = provider();
    await configureAuth(p);
    const calls = stubFetch({ "POST /api/social/session": () => json(session(false)) });
    show();
    typeEmail("ada@example.com");
    fireEvent.click(asked());
    await screen.findByLabelText("The code");

    typeCode("12345");
    expect(p.checked).toEqual([]);
    typeCode("123456");

    expect(await screen.findByText("home")).toBeTruthy();
    expect(p.checked).toEqual(["123456"]);
    expect(calls.map((call) => call.route)).toEqual(["POST /api/social/session"]);
    expect(useAuth().account()).toBe("Ada");
  });

  it("goes to the set-up for a new account", async () => {
    await configureAuth(provider());
    stubFetch({ "POST /api/social/session": () => json(session(true)) });
    show();
    typeEmail("new@example.com");
    fireEvent.click(asked());
    await screen.findByLabelText("The code");

    typeCode("123456");

    expect(await screen.findByText("welcome page")).toBeTruthy();
  });

  it("goes back to the page the person came from", async () => {
    await configureAuth(provider());
    stubFetch({ "POST /api/social/session": () => json(session(false)) });
    show("/sign-in?next=%2Fgames%2Fchess");
    typeEmail("ada@example.com");
    fireEvent.click(asked());
    await screen.findByLabelText("The code");

    typeCode("123456");

    expect(await screen.findByText("a game")).toBeTruthy();
  });

  it("says when the code is wrong and lets the person try again", async () => {
    let wrong = true;
    const p = provider();
    p.verifyCode = (code) => {
      p.checked.push(code);
      if (wrong) return Promise.reject(authError("invalid_code"));
      p.session = true;
      p.emit();
      return Promise.resolve();
    };
    await configureAuth(p);
    stubFetch({ "POST /api/social/session": () => json(session(false)) });
    show();
    typeEmail("ada@example.com");
    fireEvent.click(asked());
    await screen.findByLabelText("The code");

    typeCode("000000");
    expect((await screen.findByRole("alert")).textContent).toBe("That code is not right. Check it and try again.");

    wrong = false;
    typeCode("123456");
    expect(await screen.findByText("home")).toBeTruthy();
  });

  it("keeps only digits in the code", async () => {
    await configureAuth(provider());
    show();
    typeEmail("ada@example.com");
    fireEvent.click(asked());
    await screen.findByLabelText("The code");

    typeCode("12 3-4");

    expect((screen.getByLabelText("The code") as HTMLInputElement).value).toBe("1234");
  });

  it("goes back to the email to use another, and holds a new code back for a moment", async () => {
    const p = provider();
    await configureAuth(p);
    show();
    typeEmail("ada@example.com");
    fireEvent.click(asked());
    await screen.findByLabelText("The code");

    expect((screen.getByRole("button", { name: /Send a new code in \d+s/ }) as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(screen.getByRole("button", { name: "Use a different email" }));

    expect(await screen.findByLabelText("Your email")).toBeTruthy();
  });

  it("goes to Google, remembering where to come back to", async () => {
    const p = provider();
    await configureAuth(p);
    show("/sign-in?next=%2Fgames%2Fchess");

    fireEvent.click(await screen.findByRole("button", { name: "Continue with Google" }));

    await waitFor(() => expect(p.google_).toEqual(["/games/chess"]));
  });

  it("shows the provider's own line when it has one", async () => {
    await configureAuth(provider({ hint: "The code is 424242." }));
    show();

    expect(await screen.findByText("The code is 424242.")).toBeTruthy();
  });

  it("leaves at once for someone already signed in", async () => {
    const p = provider({ session: true });
    stubFetch({ "POST /api/social/session": () => json(session(false)) });
    await configureAuth(p);
    show();

    expect(await screen.findByText("home")).toBeTruthy();
  });
});

describe("Sign-up page", () => {
  it("is the same flow with its own words, and points to signing in", async () => {
    await configureAuth(provider());
    show("/sign-up");

    expect(await screen.findByRole("heading", { name: "Join Hubbable" })).toBeTruthy();
    expect(within(screen.getByRole("main")).getByRole("link", { name: "Sign in" }).getAttribute("href")).toBe("/sign-in");
    expect(screen.getByRole("button", { name: "Email me a code" })).toBeTruthy();
  });
});
