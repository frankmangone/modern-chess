import { For, Show, createSignal } from "solid-js";
import type { LobbySummary, VariantDetail } from "../api/types";
import { RecordTag } from "./record-tag";
import { seatColour } from "./seat-colour";

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
  /** What time it is, for how long a lobby has waited. The page's clock unless a test sets one. */
  now?: () => number;
}

/** A seat of the lobby's own variant that nobody holds. A variant the game does not list has
 *  no known seats, so it offers none. */
function freeSeat(lobby: LobbySummary, variants: VariantDetail[]): string | undefined {
  const seats = variants.find((variant) => variant.slug === lobby.variant)?.seats ?? [];
  const taken = new Set(lobby.players.map((player) => player.seat));
  return seats.find((seat) => !taken.has(seat));
}

/** How long a lobby has waited, in the biggest unit that fits. */
export function waited(createdAt: string, now: number): string {
  const minutes = Math.floor((now - Date.parse(createdAt)) / 60_000);
  if (!Number.isFinite(minutes) || minutes < 1) return "just now";
  if (minutes < 60) return `${minutes} min`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} h`;
  return `${Math.floor(hours / 24)} d`;
}

/** "WHITE" reads "White". */
function nice(seat: string): string {
  return seat.charAt(0).toUpperCase() + seat.slice(1).toLowerCase();
}

/** The play section of a game's page: open a lobby on one side, and on the other the lobbies
 *  that wait for a player, with who is in them and which seat the visitor would take. */
export function LobbyList(props: LobbyListProps) {
  const [picked, setPicked] = createSignal<string | null>(null);
  const several = () => props.variants.length > 1;
  const selected = () => picked() ?? props.preview;
  const now = () => (props.now ?? Date.now)();
  const seatsOf = (lobby: LobbySummary) =>
    props.variants.find((variant) => variant.slug === lobby.variant)?.seats ?? [];
  const yours = (lobby: LobbySummary) => lobby.players.some((player) => player.handle === props.actions.you);
  /** Yours first, then the rest as the server listed them. */
  const rows = () => [...props.lobbies].sort((a, b) => Number(yours(b)) - Number(yours(a)));

  return (
    <div class="lobbies">
      <div class="lobbies-head">
        <h2 class="lobby-title">Play a friend</h2>
      </div>
      <div class="lobbies-grid">
        <div class="lobby-open">
          <h3>Open a lobby</h3>
          <p>Open a lobby and send the link to a friend. The first to join takes the other seat.</p>
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
          <button
            class="btn-primary-lg"
            disabled={props.actions.busy}
            onClick={() => props.actions.create(selected())}
          >
            Create lobby
          </button>
          <p class="lobby-fine">No sign-up: a guest name is made for you the first time.</p>
          <Show when={props.actions.error}>
            {(message) => (
              <p class="feed-message" role="alert">
                {message()}
              </p>
            )}
          </Show>
        </div>
        <div class="lobby-waiting">
          <div class="lobby-sub">
            <h3>Waiting for a player</h3>
            <Show when={props.lobbies.length > 0}>
              <span>{props.lobbies.length} open</span>
            </Show>
          </div>
          <Show
            when={props.lobbies.length > 0}
            fallback={<p class="feed-message">No open lobbies yet. Create one and a friend can join it.</p>}
          >
            <ul class="lobby-rows">
              <For each={rows()}>
                {(lobby) => {
                  const seat = () => freeSeat(lobby, props.variants);
                  const host = () => lobby.players[0];
                  const seats = () => seatsOf(lobby);
                  return (
                    <li class="lobby-row" classList={{ "lobby-yours": yours(lobby) }} data-testid="lobby">
                      <span
                        class="lobby-avatar"
                        aria-hidden="true"
                        style={{ background: seatColour(seats(), host()?.seat ?? "") }}
                      >
                        {host()?.handle.charAt(0).toUpperCase()}
                      </span>
                      <span class="lobby-players">
                        <span class="lobby-names">
                          <For each={lobby.players}>
                            {(player) => (
                              <span class="lobby-name">
                                <b>{player.handle}</b>
                                <RecordTag record={player.record} />
                              </span>
                            )}
                          </For>
                        </span>
                        <span class="lobby-detail">
                          <Show when={several()}>
                            <span class="lobby-variant" data-testid="lobby-variant">
                              {lobby.variant_label}
                            </span>
                          </Show>
                          {lobby.players.length}/{lobby.capacity} · waiting {waited(lobby.created_at, now())}
                          <Show when={!yours(lobby) && seat()}>
                            {(free) => <> · you would play {nice(free())}</>}
                          </Show>
                        </span>
                      </span>
                      <span class="lobby-seats" aria-hidden="true">
                        <For each={seats()}>
                          {(name) => (
                            <i
                              class="lobby-pip"
                              classList={{ taken: lobby.players.some((player) => player.seat === name) }}
                              style={{ "--seat": seatColour(seats(), name) }}
                            />
                          )}
                        </For>
                      </span>
                      <Show
                        when={!yours(lobby)}
                        fallback={
                          <button class="btn-join" onClick={() => props.actions.open(lobby.id)}>
                            Open
                          </button>
                        }
                      >
                        <Show when={seat()}>
                          {(free) => (
                            <button
                              class="btn-join"
                              disabled={props.actions.busy}
                              onClick={() => props.actions.join(lobby.id, free())}
                            >
                              Join
                            </button>
                          )}
                        </Show>
                      </Show>
                    </li>
                  );
                }}
              </For>
            </ul>
          </Show>
        </div>
      </div>
    </div>
  );
}
