import { Router, Route, useNavigate } from "@solidjs/router";
import { createEffect, on } from "solid-js";
import { useAuth } from "./auth/session";
import { HomePage } from "./pages/home";
import { GameDetailPage } from "./pages/game";
import { AccountPage } from "./pages/account";
import { PlayPage } from "./pages/play";
import { RulesPage } from "./pages/rules";
import { SignInPage, SignUpPage } from "./pages/join";
import { SsoCallbackPage } from "./pages/sso-callback";
import { WelcomePage } from "./pages/welcome";

/** Takes someone to the set-up when they have just made an account. Only then: a person who comes
 *  back with the set-up unfinished is not stopped, the header says "Finish setting up". */
function SetupRedirect() {
  const auth = useAuth();
  const navigate = useNavigate();
  createEffect(
    on(
      auth.setupPending,
      (pending) => {
        if (pending) navigate("/welcome");
      },
      { defer: true },
    ),
  );
  return null;
}

export default function App() {
  return (
    <Router root={(props) => (
      <>
        <SetupRedirect />
        {props.children}
      </>
    )}>
      <Route path="/" component={HomePage} />
      <Route path="/games/:slug" component={GameDetailPage} />
      <Route path="/games/:slug/rules" component={RulesPage} />
      <Route path="/play/:id" component={PlayPage} />
      <Route path="/account" component={AccountPage} />
      <Route path="/welcome" component={WelcomePage} />
      <Route path="/sign-in" component={SignInPage} />
      <Route path="/sign-up" component={SignUpPage} />
      <Route path="/sso-callback" component={SsoCallbackPage} />
    </Router>
  );
}
