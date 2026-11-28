import { For, Show } from "solid-js";
import type { FiledGame } from "../api/types";

interface Props {
  games: FiledGame[];
  /** The game being worked on right now, so its buttons wait. */
  busy: string | null;
  onWithdraw: (game: FiledGame) => void;
  onEdit: (game: FiledGame) => void;
}

function standing(game: FiledGame): string {
  switch (game.status) {
    case "in_review":
      return "Waiting to be looked at";
    case "published":
      return game.revisions > 1 ? `On the hub, revision ${game.revisions}` : "On the hub";
    case "unpublished":
      return "Taken off the hub";
    case "removed":
      return "Removed from the hub";
    case "draft":
      return "A draft";
  }
}

/** The author's games that have left the drafts: waiting for staff, or on the hub. */
export function MyGames(props: Props) {
  return (
    <section class="builder-games" aria-label="Your games on the hub">
      <h2 class="builder-games-title">Your games</h2>
      <ul class="builder-games-list">
        <For each={props.games}>
          {(game) => (
            <li class="builder-game">
              <span class="builder-game-name">{game.name}</span>
              <span class="builder-game-standing">{standing(game)}</span>
              <span class="builder-game-actions">
                <Show when={game.status === "published" && game.slug}>
                  {(slug) => <a class="text-link" href={`/games/${slug()}`}>View on the hub</a>}
                </Show>
                <Show when={game.status === "published"}>
                  <button type="button" class="builder-small" disabled={props.busy === game.id} onClick={() => props.onEdit(game)}>
                    {game.working_copy ? "Continue editing" : "Edit"}
                  </button>
                </Show>
                <Show when={game.status === "in_review"}>
                  <button type="button" class="builder-small" disabled={props.busy === game.id} onClick={() => props.onWithdraw(game)}>
                    Take it back
                  </button>
                </Show>
              </span>
            </li>
          )}
        </For>
      </ul>
    </section>
  );
}
