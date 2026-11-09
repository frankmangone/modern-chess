import { For, Show } from "solid-js";
import type { LobbySummary } from "../api/types";

/** What the lobby buttons do, and the state they show. The page owns it. */
export interface LobbyActions {
  create: () => void;
  join: (lobbyId: string, seat: string) => void;
  /** Goes to a match the visitor already has a seat in. */
  open: (lobbyId: string) => void;
  /** The handle of the remembered guest, so their own lobbies are not offered as joinable. */
  you: string | null;
  busy: boolean;
  error: string | null;
}

interface LobbyListProps {
  lobbies: LobbySummary[];
  seats: string[];
  actions: LobbyActions;
}

function freeSeat(lobby: LobbySummary, seats: string[]): string | undefined {
  const taken = new Set(lobby.players.map((player) => player.seat));
  return seats.find((seat) => !taken.has(seat));
}

export function LobbyList(props: LobbyListProps) {
  return (
    <div class="lobbies">
      <div class="lobbies-header">
        <h2 class="lobby-title">Join a game</h2>
        <button class="btn-create-game" disabled={props.actions.busy} onClick={() => props.actions.create()}>
          Create lobby
        </button>
      </div>
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
                  <Show when={freeSeat(lobby, props.seats)}>
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
