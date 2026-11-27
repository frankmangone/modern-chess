import { A, useNavigate, useSearchParams } from "@solidjs/router";
import { Match, Show, Switch, createEffect, createSignal, onCleanup } from "solid-js";
import { continueWithGoogle, sendCode, useAuth, verifyCode } from "../auth/session";
import { AuthError, AUTH_MESSAGES } from "../auth/provider";
import Footer from "../components/Footer";
import Navbar from "../components/Navbar";
import { usePageTitle } from "../components/page-title";

/** The path to land on after signing in: the page the person came from, if it is one of ours. A
 *  full address, or one that starts `//`, could send them somewhere else, so it is ignored. */
export function safeNext(raw: string | string[] | undefined): string {
  const value = Array.isArray(raw) ? raw[0] : raw;
  if (typeof value !== "string" || !value.startsWith("/") || value.startsWith("//") || value.includes("\\")) return "/";
  return value;
}

/** How long to wait before a code can be sent again. */
const RESEND_SECONDS = 30;
const CODE_LENGTH = 6;

function reason(error: unknown): string {
  return error instanceof AuthError ? error.message : AUTH_MESSAGES.unknown;
}

export const SignInPage = () => <Join mode="in" />;
export const SignUpPage = () => <Join mode="up" />;

const COPY = {
  in: {
    title: "Welcome back",
    lead: "Sign in with Google, or with a code we email you. No password to remember.",
    card: "Sign in",
    switchText: "New here?",
    switchLink: "Make an account",
    switchTo: "/sign-up",
  },
  up: {
    title: "Join Hubbable",
    lead: "Make an account to choose a name of your own and keep your matches wherever you play.",
    card: "Make an account",
    switchText: "Already have an account?",
    switchLink: "Sign in",
    switchTo: "/sign-in",
  },
} as const;

/** Signing in and making an account are one flow: say your email, enter the code we send. Someone
 *  who has an account is signed in, anyone else is made one. The two pages only differ in what they say. */
function Join(props: { mode: "in" | "up" }) {
  const auth = useAuth();
  const navigate = useNavigate();
  const [params] = useSearchParams();
  const copy = () => COPY[props.mode];
  usePageTitle(() => copy().card);

  const next = () => safeNext(params.next);
  const [step, setStep] = createSignal<"email" | "code">("email");
  const [email, setEmail] = createSignal("");
  const [code, setCode] = createSignal("");
  const [busy, setBusy] = createSignal<"email" | "code" | "google" | null>(null);
  const [failure, setFailure] = createSignal<string | null>(null);
  const [wait, setWait] = createSignal(0);

  // Once there is an account, this page has nothing more to do.
  createEffect(() => {
    if (auth.account() !== null) navigate(auth.setupPending() ? "/welcome" : next(), { replace: true });
  });

  let timer: ReturnType<typeof setInterval> | undefined;
  onCleanup(() => clearInterval(timer));

  function startWait() {
    clearInterval(timer);
    setWait(RESEND_SECONDS);
    timer = setInterval(() => {
      setWait((seconds) => {
        if (seconds <= 1) clearInterval(timer);
        return Math.max(0, seconds - 1);
      });
    }, 1000);
  }

  async function submitEmail(event: SubmitEvent) {
    event.preventDefault();
    if (busy() !== null || email().trim() === "") return;
    setBusy("email");
    setFailure(null);
    try {
      await sendCode(email());
      setCode("");
      setStep("code");
      startWait();
    } catch (error) {
      setFailure(reason(error));
    } finally {
      setBusy(null);
    }
  }

  async function resend() {
    if (busy() !== null || wait() > 0) return;
    setBusy("email");
    setFailure(null);
    try {
      await sendCode(email());
      startWait();
    } catch (error) {
      setFailure(reason(error));
    } finally {
      setBusy(null);
    }
  }

  async function submitCode(event?: SubmitEvent) {
    event?.preventDefault();
    if (busy() !== null || code().length < CODE_LENGTH) return;
    setBusy("code");
    setFailure(null);
    try {
      await verifyCode(code());
    } catch (error) {
      setFailure(reason(error));
      setBusy(null);
    }
    // On success the page leaves (see the effect above); the button stays on "Checking…" until it does.
  }

  async function google() {
    if (busy() !== null) return;
    setBusy("google");
    setFailure(null);
    try {
      await continueWithGoogle(next());
    } catch (error) {
      setFailure(reason(error));
      setBusy(null);
    }
  }

  function typeCode(value: string) {
    const digits = value.replace(/\D/g, "").slice(0, CODE_LENGTH);
    setCode(digits);
    if (digits.length === CODE_LENGTH) void submitCode();
  }

  return (
    <div class="page">
      <Navbar />
      <main>
        <section class="setup">
          <h1 class="setup-title">{copy().title}</h1>
          <Switch>
            <Match when={!auth.ready()}>
              <p class="setup-lead setup-waiting" role="status">
                <span class="spinner" aria-hidden="true" />
                One moment…
              </p>
            </Match>
            <Match when={!auth.available()}>
              <p class="setup-lead">Signing in is not set up here. You can still play as a guest.</p>
              <A href="/" class="btn-primary-lg">
                Browse games
              </A>
            </Match>
            <Match when={true}>
              <p class="setup-lead">{copy().lead}</p>
              <div class="setup-card">
                <div class="setup-card-head">
                  <h2 class="setup-card-title">{step() === "email" ? copy().card : "Check your email"}</h2>
                </div>
                <div class="setup-card-body">
                  <Show
                    when={step() === "code"}
                    fallback={
                      <>
                        <Show when={auth.google()}>
                          <button class="btn-google" type="button" disabled={busy() !== null} aria-busy={busy() === "google"} onClick={() => void google()}>
                            <GoogleMark />
                            {busy() === "google" ? "Opening Google…" : "Continue with Google"}
                          </button>
                          <p class="join-or" aria-hidden="true">
                            or
                          </p>
                        </Show>
                        <form class="join-form" noValidate onSubmit={(event) => void submitEmail(event)}>
                          <label class="setup-label" for="email">
                            Your email
                          </label>
                          <input
                            id="email"
                            class="setup-input"
                            type="email"
                            autocomplete="email"
                            inputmode="email"
                            autocapitalize="off"
                            spellcheck={false}
                            required
                            placeholder="you@example.com"
                            value={email()}
                            aria-invalid={failure() !== null}
                            onInput={(event) => setEmail(event.currentTarget.value)}
                          />
                          <p class="setup-help">We email you a code. The same email is always the same account.</p>
                          <button class="btn-primary-lg" type="submit" disabled={busy() !== null || email().trim() === ""} aria-busy={busy() === "email"}>
                            {busy() === "email" ? "Sending…" : "Email me a code"}
                          </button>
                        </form>
                      </>
                    }
                  >
                    <form class="join-form" onSubmit={(event) => void submitCode(event)}>
                      <p>
                        We sent a {CODE_LENGTH}-digit code to <strong class="join-email">{email().trim()}</strong>.
                      </p>
                      <label class="setup-label" for="code">
                        The code
                      </label>
                      <input
                        id="code"
                        class="setup-input join-code"
                        type="text"
                        inputmode="numeric"
                        autocomplete="one-time-code"
                        autofocus
                        maxLength={CODE_LENGTH}
                        placeholder="000000"
                        value={code()}
                        aria-invalid={failure() !== null}
                        onInput={(event) => typeCode(event.currentTarget.value)}
                      />
                      <button class="btn-primary-lg" type="submit" disabled={busy() !== null || code().length < CODE_LENGTH} aria-busy={busy() === "code"}>
                        {busy() === "code" ? "Checking…" : "Continue"}
                      </button>
                      <div class="join-links">
                        <button class="join-link" type="button" onClick={() => { setStep("email"); setFailure(null); }}>
                          Use a different email
                        </button>
                        <button class="join-link" type="button" disabled={wait() > 0 || busy() !== null} onClick={() => void resend()}>
                          {wait() > 0 ? `Send a new code in ${wait()}s` : "Send a new code"}
                        </button>
                      </div>
                    </form>
                  </Show>
                  <Show when={failure()}>
                    {(message) => (
                      <p class="feed-message" role="alert">
                        {message()}
                      </p>
                    )}
                  </Show>
                  <Show when={auth.hint()}>{(hint) => <p class="setup-help join-hint">{hint()}</p>}</Show>
                  {/* Where the provider may show its bot check, if it asks for one. */}
                  <div id="clerk-captcha" />
                </div>
              </div>
              <p class="join-switch">
                {copy().switchText}{" "}
                <A href={`${copy().switchTo}${next() === "/" ? "" : `?next=${encodeURIComponent(next())}`}`}>{copy().switchLink}</A>
              </p>
            </Match>
          </Switch>
        </section>
      </main>
      <Footer />
    </div>
  );
}

/** Google's "G", in its own colours. */
function GoogleMark() {
  return (
    <svg class="google-mark" viewBox="0 0 48 48" width="20" height="20" aria-hidden="true">
      <path fill="#EA4335" d="M24 9.5c3.5 0 6.6 1.2 9.1 3.5l6.8-6.8C35.8 2.4 30.3 0 24 0 14.6 0 6.5 5.4 2.6 13.2l7.9 6.1C12.4 13.5 17.7 9.5 24 9.5z" />
      <path fill="#4285F4" d="M46.5 24.5c0-1.6-.1-3.1-.4-4.5H24v9h12.7c-.6 3-2.3 5.5-4.8 7.2l7.6 5.9c4.4-4.1 7-10.1 7-17.6z" />
      <path fill="#FBBC05" d="M10.5 28.7a14.5 14.5 0 0 1 0-9.4l-7.9-6.1a24 24 0 0 0 0 21.6l7.9-6.1z" />
      <path fill="#34A853" d="M24 48c6.5 0 11.9-2.1 15.9-5.8l-7.6-5.9c-2.1 1.4-4.9 2.3-8.3 2.3-6.3 0-11.6-4-13.5-9.8l-7.9 6.1C6.5 42.6 14.6 48 24 48z" />
    </svg>
  );
}
