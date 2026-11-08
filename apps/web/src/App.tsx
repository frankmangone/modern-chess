import { Router, Route } from "@solidjs/router";
import { HomePage } from "./pages/home";
import { GameDetailPage } from "./pages/game";

export default function App() {
  return (
    <Router>
      <Route path="/" component={HomePage} />
      <Route path="/games/:slug" component={GameDetailPage} />
    </Router>
  );
}
