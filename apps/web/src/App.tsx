import { Router, Route } from "@solidjs/router";
import { HomePage } from "./pages/home";
import { GameDetailPage } from "./pages/game";
import { PlayPage } from "./pages/play";

export default function App() {
  return (
    <Router>
      <Route path="/" component={HomePage} />
      <Route path="/games/:slug" component={GameDetailPage} />
      <Route path="/play/:id" component={PlayPage} />
    </Router>
  );
}
