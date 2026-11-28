import { Match, Switch, createEffect, createMemo, createResource, createSignal } from "solid-js";
import { type SpecCheck, checkSpec } from "./engine";
import { Playground } from "./playground";
import { createRunGuard } from "./run-guard";
import { readSkin } from "./skin";

interface Props {
  /** The game's spec, as text. */
  rules: string;
  /** The game's look, as text. */
  skin: string;
  /** Which game this is: a run that never finished drawing is remembered under it. */
  id: string;
  /** Where that is remembered. */
  guardKey: string;
  /** Told once the engine has read the rules, and what it said. */
  onChecked?: (ok: boolean) => void;
}

/** A game someone else wrote, played in this tab: the engine reads it first, and a run that did not
 *  finish drawing last time is not started again on its own, so a game that hangs the page cannot trap
 *  whoever opens it. Make one per game (key it by `id`). */
export function StrangerGame(props: Props) {
  const guard = createRunGuard(props.guardKey);
  const [verdict] = createResource(
    () => props.rules,
    (rules): Promise<SpecCheck> =>
      checkSpec(rules).catch(() => ({
        ok: false as const,
        message: "The engine could not be loaded, so this game cannot be checked. Reload the page to try again.",
        line: null,
        column: null,
        path: null,
      })),
  );
  createEffect(() => {
    const now = verdict();
    if (now) props.onChecked?.(now.ok);
  });
  const skin = createMemo(() => readSkin(props.skin));

  const [held, setHeld] = createSignal(guard.holds(props.id));
  const [started, setStarted] = createSignal(false);
  const playable = () => verdict()?.ok === true && skin().problem === null;
  const running = () => playable() && (started() || !held());
  function start() {
    setHeld(false);
    setStarted(true);
  }
  // From the moment the game is started until its board has drawn, a reload would start it again.
  createEffect(() => {
    if (running()) guard.set(props.id);
  });

  return (
    <Switch>
      <Match when={verdict.loading}>
        <p class="feed-message">Checking the game…</p>
      </Match>
      <Match when={verdict()?.ok === false ? (verdict() as Extract<SpecCheck, { ok: false }>) : null}>
        {(problem) => (
          <p class="builder-warning" role="alert">
            The engine cannot read this game, so it cannot be played. {problem().message}
          </p>
        )}
      </Match>
      <Match when={skin().problem}>
        {(problem) => (
          <p class="builder-warning" role="alert">
            The look of this game cannot be read, so it cannot be played. {problem()}
          </p>
        )}
      </Match>
      <Match when={held() && !started()}>
        <p class="builder-warning" role="alert">
          This game did not finish starting last time, so it was not started now.
        </p>
        <button type="button" class="btn-primary-lg" onClick={start}>
          Start it anyway
        </button>
      </Match>
      <Match when={running()}>
        <Playground rules={JSON.parse(props.rules) as unknown} skin={skin().skin} onAlive={() => guard.set(null)} />
      </Match>
    </Switch>
  );
}
