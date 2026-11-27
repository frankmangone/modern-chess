import { A, useNavigate } from "@solidjs/router";
import { Match, Show, Switch, createEffect, createSignal, onCleanup } from "solid-js";
import { ApiError } from "../api/http";
import type { HandleCheck } from "../api/types";
import { signUp, useAuth } from "../auth/session";
import { checkHandle, chooseHandle, finishSetup } from "../auth/setup";
import Footer from "../components/Footer";
import Navbar from "../components/Navbar";
import { usePageTitle } from "../components/page-title";
import { Stepper, type Step } from "../components/stepper";

/** The set-up a new account goes through. More steps (preferences, say) are added to this list
 *  and to the `Switch` below; the stepper and the order follow. */
const STEPS: readonly Step[] = [
  { id: "name", title: "Your name" },
  { id: "done", title: "Ready to play" },
];

/** How long to wait after the last key before asking whether the name is free. */
const TYPING_PAUSE_MS = 350;

type Verdict =
  | { state: "empty" }
  | { state: "checking" }
  | { state: "free" }
  | { state: "problem"; message: string }
  | { state: "unknown" };

export function WelcomePage() {
  const auth = useAuth();
  usePageTitle(() => "Welcome");

  const [step, setStep] = createSignal(0);

  return (
    <div class="page">
      <Navbar />
      <main>
        <section class="setup">
          <Switch>
            <Match when={auth.account() === null && auth.working() === "in"}>
              <h1 class="setup-title">Welcome to Hubbable</h1>
              <p class="setup-lead setup-waiting" role="status">
                <span class="spinner" aria-hidden="true" />
                Getting your account ready…
              </p>
            </Match>
            <Match when={auth.account() === null}>
              <h1 class="setup-title">Welcome to Hubbable</h1>
              <p class="setup-lead">Make an account to choose a name of your own, and keep your matches wherever you play.</p>
              <Show when={auth.available()}>
                <button class="btn-primary-lg" type="button" onClick={signUp}>
                  Sign up
                </button>
              </Show>
            </Match>
            <Match when={!auth.setupPending() && step() === 0}>
              <h1 class="setup-title">You are all set</h1>
              <p class="setup-lead">There is nothing left to set up.</p>
              <A href="/" class="btn-primary-lg">
                Browse games
              </A>
            </Match>
            <Match when={true}>
              <h1 class="setup-title">Welcome to Hubbable</h1>
              <p class="setup-lead">Choose a name, and you can play.</p>
              <Stepper steps={STEPS} current={step()} label="Account set-up" />
              <Switch>
                <Match when={step() === 0}>
                  <NameStep onDone={() => setStep(1)} />
                </Match>
                <Match when={step() === 1}>
                  <DoneStep />
                </Match>
              </Switch>
            </Match>
          </Switch>
        </section>
      </main>
      <Footer />
    </div>
  );
}

/** Choosing a username. The server says whether a name is allowed and free, as the person types;
 *  the same rules run again when they continue, so this never decides anything. */
function NameStep(props: { onDone: () => void }) {
  const auth = useAuth();
  const [name, setName] = createSignal("");
  const [verdict, setVerdict] = createSignal<Verdict>({ state: "empty" });
  const [busy, setBusy] = createSignal(false);
  const [refusal, setRefusal] = createSignal<string | null>(null);

  // Ask about the name once typing pauses. An answer for a name that has since changed is dropped.
  createEffect(() => {
    const asked = name().trim();
    setRefusal(null);
    if (asked === "") {
      setVerdict({ state: "empty" });
      return;
    }
    setVerdict({ state: "checking" });
    const timer = setTimeout(() => {
      checkHandle(asked).then(
        (answer: HandleCheck) => {
          if (name().trim() !== asked) return;
          setVerdict(answer.available ? { state: "free" } : { state: "problem", message: answer.problem?.message ?? "That name cannot be used." });
        },
        () => {
          if (name().trim() === asked) setVerdict({ state: "unknown" });
        },
      );
    }, TYPING_PAUSE_MS);
    onCleanup(() => clearTimeout(timer));
  });

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    if (busy() || name().trim() === "") return;
    setBusy(true);
    setRefusal(null);
    try {
      await chooseHandle(name().trim());
      props.onDone();
    } catch (error) {
      setRefusal(error instanceof ApiError ? error.message : "That did not work. Please try again.");
      setBusy(false);
    }
  }

  const verdictText = () => {
    const current = verdict();
    switch (current.state) {
      case "checking":
        return "Checking…";
      case "free":
        return (
          <>
            <strong>{name().trim()}</strong> is free.
          </>
        );
      case "problem":
        return current.message;
      default:
        return null;
    }
  };

  const canContinue = () => !busy() && (verdict().state === "free" || verdict().state === "unknown");

  return (
    <form class="setup-card" onSubmit={(event) => void submit(event)}>
      <div class="setup-card-head">
        <h2 class="setup-card-title">Choose your username</h2>
      </div>
      <div class="setup-card-body">
        <label class="setup-label" for="username">
          The name other players see at the board
        </label>
        <input
          id="username"
          class="setup-input"
          type="text"
          autocomplete="username"
          autocapitalize="off"
          spellcheck={false}
          maxLength={24}
          placeholder={auth.account() ?? ""}
          value={name()}
          aria-describedby="username-help username-verdict"
          aria-invalid={verdict().state === "problem" || refusal() !== null}
          onInput={(event) => setName(event.currentTarget.value)}
        />
        <p class="setup-help" id="username-help">
          3 to 24 letters, numbers, dashes or underscores. It starts with a letter or a number.
        </p>
        <p class="setup-verdict" id="username-verdict" data-state={verdict().state} role="status">
          {verdictText()}
        </p>
        <Show when={refusal()}>
          {(message) => (
            <p class="feed-message" role="alert">
              {message()}
            </p>
          )}
        </Show>
        <div class="game-actions">
          <button class="btn-primary-lg" type="submit" disabled={!canContinue()} aria-busy={busy()}>
            {busy() ? "Saving…" : "Continue"}
          </button>
        </div>
      </div>
    </form>
  );
}

/** The last step: a welcome, and the button that ends the set-up. */
function DoneStep() {
  const auth = useAuth();
  const navigate = useNavigate();
  const [busy, setBusy] = createSignal(false);
  const [failed, setFailed] = createSignal(false);

  async function start() {
    setBusy(true);
    setFailed(false);
    try {
      await finishSetup();
      navigate("/");
    } catch {
      setFailed(true);
      setBusy(false);
    }
  }

  return (
    <div class="setup-card">
      <div class="setup-card-head">
        <h2 class="setup-card-title">You are in, {auth.account()}</h2>
      </div>
      <div class="setup-card-body">
        <p>Pick a game, open a lobby, and send the link to a friend. Your matches are kept under this name.</p>
        <Show when={failed()}>
          <p class="feed-message" role="alert">
            That did not work. Please try again.
          </p>
        </Show>
        <div class="game-actions">
          <button class="btn-primary-lg" type="button" disabled={busy()} aria-busy={busy()} onClick={() => void start()}>
            {busy() ? "One moment…" : "Start playing"}
          </button>
        </div>
      </div>
    </div>
  );
}
