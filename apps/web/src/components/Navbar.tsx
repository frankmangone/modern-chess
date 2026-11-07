import { A } from "@solidjs/router";

interface NavbarProps {
  activeLink?: "games" | "community" | "learn";
}

export default function Navbar(props: NavbarProps) {
  return (
    <nav class="navbar">
      <A href="/" class="navbar-logo">
        <div class="navbar-logo-mark">♟</div>
        <span class="navbar-logo-text">BoardHub</span>
      </A>

      <div class="navbar-links">
        <A href="/" class={`navbar-link ${props.activeLink === "games" ? "active" : ""}`}>
          Games
        </A>
        <A href="/" class={`navbar-link ${props.activeLink === "community" ? "active" : ""}`}>
          Community
        </A>
        <A href="/" class={`navbar-link ${props.activeLink === "learn" ? "active" : ""}`}>
          Learn
        </A>
      </div>

      <div class="navbar-right">
        <button class="btn-login">Log in</button>
        <button class="btn-signup">Sign up</button>
      </div>
    </nav>
  );
}
