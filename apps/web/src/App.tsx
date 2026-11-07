import { Router, Route } from "@solidjs/router";
import HomePage from "./pages/HomePage";
import GameDetailPage from "./pages/GameDetailPage";

export default function App() {
  return (
    <Router>
      <Route path="/" component={HomePage} />
      <Route path="/games/:slug" component={GameDetailPage} />
    </Router>
  );
}
