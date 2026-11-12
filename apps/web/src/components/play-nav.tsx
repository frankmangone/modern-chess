import { A } from "@solidjs/router";

interface PlayNavProps {
  /** The game's slug, for the way back. */
  game: string;
  /** What the way back is called: the game's name. */
  name: string;
  /** What is being played: the game, and its variant when the game has several. */
  title: string;
  /** One sentence about where the match stands. */
  status: string;
  /** It is the viewer's move, so the sentence should stand out. */
  yours: boolean;
}

/** The slim bar across the top of a match: the way back, what is being played, how it stands. */
export function PlayNav(props: PlayNavProps) {
  return (
    <header class="play-nav">
      <A href={`/games/${props.game}`} class="play-back">
        <span aria-hidden="true">←</span> {props.name}
      </A>
      <span class="play-nav-divider" aria-hidden="true" />
      <h1 class="play-nav-title">
        <span class="play-nav-icon" aria-hidden="true">♟</span>
        {props.title} — Match
      </h1>
      <p class="play-status" classList={{ "play-status-yours": props.yours }} role="status">
        <span class="play-status-dot" aria-hidden="true" />
        {props.status}
      </p>
    </header>
  );
}
