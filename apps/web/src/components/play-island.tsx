import { For, Show } from "solid-js";
import type { Answer, BagView, BoardView } from "../api/types";
import { showsBoard } from "./board-panel";
import { PieceFace } from "./piece-face";

export interface PlayIslandProps {
  view: BoardView;
  skin: Record<string, unknown> | null;
  /** The viewer's seat, or null for someone only watching. */
  seat: string | null;
  /** True when the viewer may act right now. */
  interactive: boolean;
  /** The match is over whatever the board says (a resignation does not change the board's status). */
  finished?: boolean;
  /** Answers the open choice: a code, a held piece or an action. */
  onChoose: (answer: Answer) => void;
  /** Takes back what was chosen so far. */
  onCancel: () => void;
}

/** "PASS" on a button reads "Pass". */
function actionLabel(name: string): string {
  const text = name.replaceAll("_", " ").toLowerCase();
  return text.charAt(0).toUpperCase() + text.slice(1);
}

const HAND = "HAND";

/** Wording a game's skin may supply, in the skin's `labels`: what its bags are called (`{owner}`
 *  stands for whose it is; `false` leaves a bag out), what its actions say on their buttons, what
 *  to tell the player (under `held` for choosing a card from the hand, or an action's name for a
 *  step that is only a button), the order the bags are listed in (a list: the stored skin does not
 *  keep the order of an object's keys), and whether a piece's code is written beside its face. */
interface Labels {
  order: string[];
  bags: Record<string, string | false>;
  actions: Record<string, string>;
  hints: Record<string, string>;
  pieceCodes: boolean;
}

function labelsOf(skin: Record<string, unknown> | null): Labels {
  const raw = skin?.labels;
  const labels: Labels = { order: [], bags: {}, actions: {}, hints: {}, pieceCodes: true };
  if (typeof raw !== "object" || raw === null) return labels;
  const given = raw as Record<string, unknown>;
  for (const [key, into] of [["bags", labels.bags], ["actions", labels.actions], ["hints", labels.hints]] as const) {
    const table = given[key];
    if (typeof table !== "object" || table === null) continue;
    for (const [name, text] of Object.entries(table)) {
      if (typeof text === "string" || (key === "bags" && text === false)) (into as Record<string, unknown>)[name] = text;
    }
  }
  if (Array.isArray(given.order)) labels.order = given.order.filter((name): name is string => typeof name === "string");
  if (given.piece_codes === false) labels.pieceCodes = false;
  return labels;
}

/** The most card backs drawn in a stack: past it the count says the rest. */
const MAX_BACKS = 5;

/** What the tray calls a bag: a hand is "Your hand" to its owner, a store by its name. */
function bagLabel(bag: BagView, seat: string | null, labels: Labels): string {
  if (bag.name === HAND) return bag.owner === seat ? "Your hand" : `${bag.owner ?? ""}'s hand`;
  const named = labels.bags[bag.name];
  if (typeof named === "string") return named.replaceAll("{owner}", bag.owner === seat ? "Your" : `${bag.owner ?? ""}'s`);
  return bag.owner === undefined ? bag.name : `${bag.owner}'s ${bag.name}`;
}

/** The part of the play page that changes with the moment: the promotion choice, the bags (the
 *  hands among them) and a line saying what to do next. */
export function PlayIsland(props: PlayIslandProps) {
  const choice = () => props.view.choice;
  const labels = () => labelsOf(props.skin);
  const promoting = () => props.interactive && choice().codes.length > 0;
  /** A code choice that comes after a piece and a square were answered is a promotion; one that
   *  comes first (rock, paper or scissors) is just a choice between the codes. */
  const promotion = () => choice().chosen.some((answer) => "place" in answer);
  /** The held piece chosen so far this turn, if any. */
  const chosenHeld = () => {
    for (const answer of choice().chosen) if ("held" in answer) return answer.held;
    return null;
  };
  /**
   * The bags worth showing. The viewer's own hand comes first, then the other hands in turn
   * order, then the rest by name. A hand with nothing in it is only worth showing to its owner.
   */
  const bags = () => {
    const { players, bags: all } = props.view;
    const worthShowing = (bag: BagView) =>
      labels().bags[bag.name] === false
        ? false
        : bag.hidden === true
        ? bag.count !== 0
        : bag.name !== HAND || bag.owner === props.seat || Object.keys(bag.pieces).length > 0;
    const rank = (bag: BagView) => {
      if (bag.name !== HAND) return players.length + 1;
      if (bag.owner === props.seat) return -1;
      return players.indexOf(bag.owner ?? "");
    };
    // A skin that lists its bags names the order they should be read in: the table first, then the
    // hands. A bag it does not list comes last. Without a list every bag ranks the same, and the
    // hands-first order above decides.
    const listed = labels().order;
    const place = (bag: BagView) => {
      const index = listed.indexOf(bag.name);
      return index < 0 ? listed.length : index;
    };
    return all.filter(worthShowing).sort((a, b) => place(a) - place(b) || rank(a) - rank(b));
  };
  /** The ring is for the player who has something to do; everyone else sees it quiet. */
  const yourTurn = () => props.seat !== null && props.view.turn === props.seat;
  /**
   * The actions worth a button: those that play at once (a pass), and the ones that only narrow
   * the choice once something is answered, or when several supplies compete for the same
   * place. At the start of a turn "Move" or "Stone" would only be noise.
   */
  const actions = () => {
    if (!props.interactive || promoting()) return [];
    const { actions: all, chosen, places } = choice();
    const narrowing = all.filter((action) => !action.plays);
    const competing = narrowing.length > 1 && places.some((place) => place.for === "PLACE");
    return all.filter((action) => action.plays || chosen.length > 0 || competing);
  };
  const scores = () => Object.entries(props.view.score);
  /** The game is decided: what each side held is shown (a hidden bag is told at the end), and there is
   *  nothing left to say about what to do next. */
  const over = () => props.finished === true || props.view.status.state !== "playing";
  /** Whether this bag is the one the chosen held piece comes from: the first one listing its code. */
  const isChosenFrom = (bag: BagView, code: string) => {
    if (chosenHeld() !== code) return false;
    const source = bags().find((other) => (other.pieces[code] ?? 0) > 0);
    return source !== undefined && source.name === bag.name && source.owner === bag.owner;
  };
  /**
   * A viewer's own bag is always theirs to pick from. Any other bag (shared, or an opponent's)
   * only when the open choice asks for a held piece of this code.
   */
  const canPick = (bag: BagView, code: string) =>
    props.interactive &&
    !promoting() &&
    (bag.owner === props.seat || choice().held.includes(code) || isChosenFrom(bag, code));

  const hint = () => {
    const { view } = props;
    if (!props.interactive) return props.seat === null ? `${view.turn} to move.` : `Waiting for ${view.turn}…`;
    if (promoting()) return promotion() ? "Choose a promotion." : "Choose one.";
    // A game's skin may say what its places are for, where the built-in line would be wrong: the
    // points of Backgammon are where a die takes a checker, and not empty points to place on.
    const skinHint = props.skin?.hint;
    if (
      typeof skinHint === "string" &&
      choice().chosen.length === 0 &&
      choice().start === null &&
      choice().places.length > 0
    ) {
      return skinHint;
    }
    // A game that says what to do with its cards or its buttons says it before the built-in lines.
    const said = labels().hints;
    if (choice().chosen.length === 0 && choice().start === null && choice().places.length === 0) {
      if (choice().held.length > 0 && said.held !== undefined) return said.held;
      const buttons = choice().actions.filter((action) => action.plays);
      if (choice().held.length === 0 && buttons.length === 1 && said[buttons[0].name] !== undefined) return said[buttons[0].name];
    }
    const held = chosenHeld();
    if (held !== null) return `${held} selected. Choose a highlighted square.`;
    if (choice().start !== null) return "Piece selected. Choose a highlighted square.";
    const narrowing = choice().actions.filter((action) => !action.plays);
    const placing = choice().places.some((place) => place.for === "PLACE");
    if (choice().chosen.length === 0 && narrowing.length > 0 && placing) {
      const aPiece = narrowing.length === 1 ? `a ${narrowing[0].name.toLowerCase()}` : "a piece";
      const passed = view.passes > 0 ? "The last turn was a pass. " : "";
      const canPass = choice().actions.some((action) => action.plays);
      return `${passed}Place ${aPiece} on an empty point${canPass ? ", or pass" : ""}.`;
    }
    if (choice().chosen.length > 0) return "Choose a highlighted square.";
    const hand = view.bags.find((bag) => bag.name === HAND && bag.owner === props.seat);
    if (hand !== undefined && Object.keys(hand.pieces).length > 0) {
      return "Move a piece, or drop one you hold.";
    }
    // Ways that play at once (agreeing to a count, marking done) are buttons: say so when there is
    // nothing to move, or when a piece is also asked for (marking a stone as dead).
    const playsNow = choice().actions.some((action) => action.plays);
    if (playsNow) return choice().places.length > 0 ? "Select a piece, or choose an action." : "Choose an action.";
    return "Select a piece to see where it can go.";
  };

  return (
    <section class="island" classList={{ "island-quiet": !yourTurn(), "island-table": !showsBoard(props.skin) }} aria-label="What to do next">
      <Show when={promoting()}>
        <header class="island-head">
          <span class="island-pulse" aria-hidden="true" />
          <span class="island-title">{promotion() ? "Action required" : "Your choice"}</span>
        </header>
        <div class="island-choices">
          <For each={choice().codes}>
            {(code) => (
              <button
                type="button"
                class="island-choice"
                aria-label={`${promotion() ? "Promote to" : "Choose"} ${code}`}
                onClick={() => props.onChoose({ code })}
              >
                <PieceFace skin={props.skin} code={code} player={props.view.turn} players={props.view.players} />
                <span class="island-choice-name">{code}</span>
              </button>
            )}
          </For>
        </div>
      </Show>

      <For each={bags()}>
        {(bag) => {
          const held = () => Object.entries(bag.pieces);
          const label = () => bagLabel(bag, props.seat, labels());
          const owner = bag.owner ?? "";
          const backs = () => Math.min(bag.count ?? 1, MAX_BACKS);
          return (
            <div class="island-hand">
              <p class="island-hand-label">{label()}</p>
              <Show when={bag.hidden !== true} fallback={
                <div
                  class="island-stack"
                  role="img"
                  aria-label={bag.count === undefined ? `${label()}: hidden` : `${label()}: ${bag.count} hidden`}
                >
                  <div class="island-stack-backs" aria-hidden="true">
                    <For each={Array.from({ length: backs() }, (_, index) => index)}>
                      {() => <span class="island-card-back" />}
                    </For>
                  </div>
                  <span class="island-stack-count">{bag.count === undefined ? "Hidden" : `×${bag.count}`}</span>
                </div>
              }>
              <Show
                when={held().length > 0}
                fallback={
                  <p class="island-hand-empty">
                    {bag.name === HAND
                      ? "Nothing in hand"
                      : typeof labels().bags[bag.name] === "string"
                        ? "Nothing yet"
                        : `Nothing in ${bag.name.toLowerCase()}`}
                  </p>
                }
              >
                <div class="island-chips">
                  <For each={held()}>
                    {([code, count]) => {
                      const face = () => (
                        <>
                          <PieceFace skin={props.skin} code={code} player={owner} players={props.view.players} />
                          <Show when={labels().pieceCodes || count > 1}>
                            <span class="island-chip-label">
                              {labels().pieceCodes ? `${code} ×${count}` : `×${count}`}
                            </span>
                          </Show>
                        </>
                      );
                      return (
                        <Show
                          when={canPick(bag, code)}
                          fallback={<span class="island-chip island-chip-static">{face()}</span>}
                        >
                          <button
                            type="button"
                            class="island-chip"
                            classList={{ "island-chip-chosen": isChosenFrom(bag, code) }}
                            aria-label={`${bag.name === HAND && bag.owner === props.seat ? "Drop" : "Choose"} ${code}`}
                            aria-pressed={isChosenFrom(bag, code)}
                            disabled={chosenHeld() !== code && !choice().held.includes(code) && choice().chosen.length === 0}
                            onClick={() => (isChosenFrom(bag, code) ? props.onCancel() : props.onChoose({ held: code }))}
                          >
                            {face()}
                          </button>
                        </Show>
                      );
                    }}
                  </For>
                </div>
              </Show>
              </Show>
            </div>
          );
        }}
      </For>

      <Show when={scores().length > 0}>
        <div class="island-score">
          <p class="island-hand-label">{over() ? "Final count" : "Count if the game ended now"}</p>
          <For each={scores()}>
            {([player, points]) => (
              <p class="island-score-row">
                <span>{player}</span>
                <span class="island-score-points">{points}</span>
              </p>
            )}
          </For>
        </div>
      </Show>

      <Show when={!over()}>
        <p class="island-hint">{hint()}</p>
      </Show>

      <For each={actions()}>
        {(action) => (
          <button type="button" class="island-pass" onClick={() => props.onChoose({ action: action.name })}>
            {labels().actions[action.name] ?? actionLabel(action.name)}
          </button>
        )}
      </For>
    </section>
  );
}
