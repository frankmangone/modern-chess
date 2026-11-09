import { A } from "@solidjs/router";

interface PlayNavProps {
  slug: string;
  name: string;
  /** One sentence about where the match stands. */
  status: string;
}

/** The slim bar across the top of a match: the way back, what is being played, how it stands. */
export function PlayNav(props: PlayNavProps) {
  return (
    <header class="play-nav">
      <A href={`/games/${props.slug}`} class="play-back">
        <span aria-hidden="true">←</span> {props.name}
      </A>
      <span class="play-nav-divider" aria-hidden="true" />
      <h1 class="play-nav-title">
        <span class="play-nav-icon" aria-hidden="true">♟</span>
        {props.name} — Match
      </h1>
      <p class="play-status" role="status">
        <span class="play-status-dot" aria-hidden="true" />
        {props.status}
      </p>
    </header>
  );
}
