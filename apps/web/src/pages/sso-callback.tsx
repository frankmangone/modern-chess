import { A, useNavigate } from "@solidjs/router";
import { Match, Switch, createEffect, createSignal } from "solid-js";
import { AuthError, AUTH_MESSAGES } from "../auth/provider";
import { completeRedirect, useAuth } from "../auth/session";
import Footer from "../components/Footer";
import Navbar from "../components/Navbar";
import { usePageTitle } from "../components/page-title";

/** Where Google sends the person back to. It finishes the trip, then moves on: the set-up for a
 *  new account, or the page they came from. */
export function SsoCallbackPage() {
  const auth = useAuth();
  const navigate = useNavigate();
  const [failure, setFailure] = createSignal<string | null>(null);
  usePageTitle(() => "Signing in");

  // Once the provider is loaded, finish the trip, once.
  let started = false;
  createEffect(() => {
    if (!auth.ready() || !auth.available() || started) return;
    started = true;
    completeRedirect().then(
      (path) => navigate(auth.setupPending() ? "/welcome" : path, { replace: true }),
      (error: unknown) => setFailure(error instanceof AuthError ? error.message : AUTH_MESSAGES.unknown),
    );
  });

  return (
    <div class="page">
      <Navbar />
      <main>
        <section class="setup">
          <Switch>
            <Match when={failure()}>
              {(message) => (
                <>
                  <h1 class="setup-title">That did not work</h1>
                  <p class="setup-lead" role="alert">
                    {message()}
                  </p>
                  <A href="/sign-in" class="btn-primary-lg">
                    Try again
                  </A>
                </>
              )}
            </Match>
            <Match when={true}>
              <h1 class="setup-title">Signing you in</h1>
              <p class="setup-lead setup-waiting" role="status">
                <span class="spinner" aria-hidden="true" />
                One moment…
              </p>
              <div id="clerk-captcha" />
            </Match>
          </Switch>
        </section>
      </main>
      <Footer />
    </div>
  );
}
