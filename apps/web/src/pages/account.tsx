import { A, useNavigate } from "@solidjs/router";
import { Show, createSignal } from "solid-js";
import { deleteAccount, signOut, useAuth } from "../auth/session";
import Footer from "../components/Footer";
import Navbar from "../components/Navbar";
import { usePageTitle } from "../components/page-title";

/** Who is signed in, with the two things to do about it: sign out, or delete the account. */
export function AccountPage() {
  const auth = useAuth();
  const navigate = useNavigate();
  const [confirming, setConfirming] = createSignal(false);
  const [busy, setBusy] = createSignal(false);
  const [failed, setFailed] = createSignal(false);
  usePageTitle(() => "Your account");

  async function leave(action: () => Promise<void>) {
    setBusy(true);
    setFailed(false);
    try {
      await action();
      navigate("/");
    } catch {
      setFailed(true);
      setBusy(false);
    }
  }

  return (
    <div class="page">
      <Navbar />
      <main>
        <section class="account-page">
          <h1 class="game-page-title">Your account</h1>
          <Show
            when={auth.account()}
            fallback={
              <>
                <p class="feed-message">
                  {auth.available() ? "You are playing as a guest." : "Signing in is not set up here."}
                </p>
                <Show when={auth.available()}>
                  <A href="/sign-in" class="btn-signup">
                    Sign in
                  </A>
                </Show>
              </>
            }
          >
            {(handle) => (
              <>
                <p class="account-handle">
                  Signed in as <strong>{handle()}</strong>.
                </p>
                <div class="game-actions">
                  <button class="btn-login" type="button" disabled={busy()} onClick={() => void leave(signOut)}>
                    Sign out
                  </button>
                </div>

                <h2 class="account-danger-title">Delete your account</h2>
                <p>
                  This removes your sign-in and your email address from
                  Hubbable. Matches you finished stay, under a name that is no longer yours. It cannot be undone.
                </p>
                <Show
                  when={confirming()}
                  fallback={
                    <button class="play-resign" type="button" onClick={() => setConfirming(true)}>
                      Delete my account
                    </button>
                  }
                >
                  <div class="game-actions">
                    <button class="play-resign-confirm" type="button" disabled={busy()} onClick={() => void leave(deleteAccount)}>
                      Yes, delete it
                    </button>
                    <button class="play-resign-keep" type="button" disabled={busy()} onClick={() => setConfirming(false)}>
                      Keep it
                    </button>
                  </div>
                </Show>
                <Show when={failed()}>
                  <p class="feed-message" role="alert">
                    That did not work. Please try again.
                  </p>
                </Show>
              </>
            )}
          </Show>
        </section>
      </main>
      <Footer />
    </div>
  );
}
