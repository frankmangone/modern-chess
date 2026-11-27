import { A, useLocation } from "@solidjs/router";
import { Show } from "solid-js";
import { signOut, useAuth } from "../auth/session";

interface NavbarProps {
  activeLink?: "games" | "community" | "learn";
}

export default function Navbar(props: NavbarProps) {
  const auth = useAuth();
  const location = useLocation();
  // Signing in brings the person back to the page they were on.
  const next = () =>
    location.pathname === "/" || location.pathname.startsWith("/sign-") || location.pathname === "/sso-callback"
      ? ""
      : `?next=${encodeURIComponent(location.pathname + location.search)}`;

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
          {/* While the trade for our token (or the sign-out) runs, one calm button says so, in place
              of buttons that would flip to something else a moment later. */}
          <Show
            when={auth.working() === null}
            fallback={
              <button class="btn-login" type="button" disabled aria-busy="true">
                <span class="spinner" aria-hidden="true" />
                {auth.working() === "out" ? "Signing out…" : "Signing in…"}
              </button>
            }
          >
            <Show
              when={auth.account()}
              fallback={
                <>
                  <A href={`/sign-in${next()}`} class="btn-login">
                    Sign in
                  </A>
                  <A href={`/sign-up${next()}`} class="btn-signup">
                    Sign up
                  </A>
                </>
              }
            >
              {(handle) => (
                <>
                  <Show when={auth.setupPending() && location.pathname !== "/welcome"}>
                    <A href="/welcome" class="btn-signup">
                      Finish setting up
                    </A>
                  </Show>
                  <A href="/account" class="navbar-handle">
                    {handle()}
                  </A>
                  <button class="btn-login" type="button" onClick={() => void signOut()}>
                    Sign out
                  </button>
                </>
              )}
            </Show>
          </Show>
        </div>
      </Show>
    </nav>
  );
}
