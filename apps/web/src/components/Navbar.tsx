import { A } from "@solidjs/router";
import { Show } from "solid-js";
import { signIn, signOut, useAuth } from "../auth/session";

interface NavbarProps {
  activeLink?: "games" | "community" | "learn";
}

export default function Navbar(props: NavbarProps) {
  const auth = useAuth();

  return (
    <nav class="navbar">
      <A href="/" class="navbar-logo">
        <div class="navbar-logo-mark">♟</div>
        <span class="navbar-logo-text">Hubbable</span>
      </A>

      <div class="navbar-links">
        <A href="/" class={`navbar-link ${props.activeLink === "games" ? "current" : ""}`}>
          Games
        </A>
        <A href="/" class={`navbar-link ${props.activeLink === "community" ? "current" : ""}`}>
          Community
        </A>
        <A href="/" class={`navbar-link ${props.activeLink === "learn" ? "current" : ""}`}>
          Learn
        </A>
      </div>

      {/* With no sign-in set up there is nothing to offer, so nothing is shown. */}
      <Show when={auth.available()}>
        <div class="navbar-right">
          <Show
            when={auth.account()}
            fallback={
              <button class="btn-signup" type="button" onClick={signIn}>
                Sign in
              </button>
            }
          >
            {(handle) => (
              <>
                <A href="/account" class="navbar-handle">
                  {handle()}
                </A>
                <button class="btn-login" type="button" onClick={() => void signOut()}>
                  Sign out
                </button>
              </>
            )}
          </Show>
        </div>
      </Show>
    </nav>
  );
}
