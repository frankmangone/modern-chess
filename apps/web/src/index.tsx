import { render } from "solid-js/web";
import App from "./App";
import { createProvider } from "./auth/config";
import { configureAuth } from "./auth/session";
import "./styles.css";

render(() => <App />, document.getElementById("root")!);

// Sign-in loads after the page is up: the page never waits on a provider, and if it cannot be
// reached everyone simply plays as a guest.
void createProvider()
  .then(configureAuth)
  .catch((error: unknown) => console.warn("Sign-in could not start:", error));
