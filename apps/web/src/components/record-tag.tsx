import { Show } from "solid-js";
import type { GameRecord } from "../api/types";

/** "12 wins, 3 draws, 5 losses": the record as a sentence, for whoever cannot see the short form. */
export function recordSentence(record: GameRecord): string {
  const part = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;
  return [part(record.won, "win", "wins"), part(record.drawn, "draw", "draws"), part(record.lost, "loss", "losses")].join(", ");
}

/** What a person has done on a game, short enough to sit beside their name: "12W 3D 5L", or "New"
 *  for someone who has finished nothing yet. */
export function RecordTag(props: { record: GameRecord }) {
  const played = () => props.record.won + props.record.drawn + props.record.lost;
  return (
    <span class="record-tag">
      <Show
        when={played() > 0}
        fallback={
          <>
            <span aria-hidden="true">New</span>
            <span class="visually-hidden">No finished matches on this game yet</span>
          </>
        }
      >
        <span aria-hidden="true">
          {props.record.won}W {props.record.drawn}D {props.record.lost}L
        </span>
        <span class="visually-hidden">{recordSentence(props.record)}</span>
      </Show>
    </span>
  );
}
