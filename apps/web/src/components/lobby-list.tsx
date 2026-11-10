import { For, Show, createSignal } from "solid-js";
import type { LobbySummary, VariantDetail } from "../api/types";

/** What the lobby buttons do, and the state they show. The page owns it. */
export interface LobbyActions {
  /** Opens a lobby in the variant with this slug. */
  create: (variant: string) => void;
  join: (lobbyId: string, seat: string) => void;
  /** Goes to a match the visitor already has a seat in. */
  open: (lobbyId: string) => void;
  /** The handle of the remembered guest, so their own lobbies are not offered as joinable. */
  you: string | null;
  busy: boolean;
  error: string | null;
}

interface LobbyListProps {
  /** Open lobbies of every variant of the game. */
  lobbies: LobbySummary[];
  variants: VariantDetail[];
  /** The slug of the variant the picker starts on. */
  preview: string;
  actions: LobbyActions;
}

/** A seat of the lobby's own variant that nobody holds. A variant the game does not list has
 *  no known seats, so it offers none. */
function freeSeat(lobby: LobbySummary, variants: VariantDetail[]): string | undefined {
  const seats = variants.find((variant) => variant.slug === lobby.variant)?.seats ?? [];
  const taken = new Set(lobby.players.map((player) => player.seat));
  return seats.find((seat) => !taken.has(seat));
}

export function LobbyList(props: LobbyListProps) {
  const [picked, setPicked] = createSignal<string | null>(null);
  const several = () => props.variants.length > 1;
  const selected = () => picked() ?? props.preview;

  return (
    <div class="lobbies">
      <div class="lobbies-header">
        <h2 class="lobby-title">Play</h2>
        <button
          class="btn-create-game"
          disabled={props.actions.busy}
          onClick={() => props.actions.create(selected())}
        >
          Create lobby
        </button>
      </div>
      <Show when={several()}>
        <div class="variant-picker" role="radiogroup" aria-label="Variant">
          <For each={props.variants}>
            {(variant) => (
              <button
                class="variant-option"
                role="radio"
                aria-checked={selected() === variant.slug}
                onClick={() => setPicked(variant.slug)}
              >
                {variant.label}
              </button>
            )}
          </For>
        </div>
      </Show>
      <Show when={props.actions.error}>
        {(message) => (
          <p class="feed-message" role="alert">
            {message()}
          </p>
        )}
      </Show>
      <Show
        when={props.lobbies.length > 0}
        fallback={<p class="feed-message">No open lobbies yet. Create one and a friend can join it.</p>}
      >
        <ul class="lobby-rows">
          <For each={props.lobbies}>
            {(lobby) => (
              <li class="lobby-row" data-testid="lobby">
                <span class="lobby-players">
                  <Show when={several()}>
                    <span class="lobby-variant" data-testid="lobby-variant">
                      {lobby.variant_label}
                    </span>
                  </Show>
                  {lobby.players.map((player) => player.handle).join(", ")} · {lobby.players.length}/
                  {lobby.capacity}
                </span>
                <Show
                  when={!lobby.players.some((player) => player.handle === props.actions.you)}
                  fallback={
                    <button class="btn-join" onClick={() => props.actions.open(lobby.id)}>
                      Open
                    </button>
                  }
                >
                  <Show when={freeSeat(lobby, props.variants)}>
                    {(seat) => (
                      <button
                        class="btn-join"
                        disabled={props.actions.busy}
                        onClick={() => props.actions.join(lobby.id, seat())}
                      >
                        Join
                      </button>
                    )}
                  </Show>
                </Show>
              </li>
            )}
          </For>
        </ul>
      </Show>
    </div>
  );
}
