import { A } from "@solidjs/router";
import { Show } from "solid-js";
import { Dynamic } from "solid-js/web";
import type { CatalogEntry } from "../api/types";
import { showsBoard } from "./board-panel";
import { CardFan, showcaseOf } from "./card-fan";
import { GameBoard } from "./game-board";

interface CatalogCardProps {
  entry: CatalogEntry;
  /** The level of the game's name: one below whatever heads the list the card sits in. */
  headingLevel?: 2 | 3 | 4;
  /** How many games are variants of this one: a link to them on the game's page. */
  variantCount?: number;
  /** Whether the kind of game is named on the card. A list whose games are all one kind, as the
   *  variants of a game are, leaves it out. */
  showKind?: boolean;
  /** The featured band's sizes: the lead has the way in on it, a side card puts its board beside
   *  its words. Left out, the card is the plain one of the grid. */
  size?: "lead" | "side";
}

const FALLBACK_ACCENT = "#8B4513";

/** The tag that says what kind of game it is. A tag that only counts the players says nothing
 *  about the game, so it is not one. */
export function genreTags(entry: CatalogEntry): string[] {
  return entry.tags.filter((tag) => tag !== "two players");
}

/** One game: its board as a game starts, straight on flat paper, then its name and one line. The
 *  name is the link, stretched over the whole card; the link to its variants sits above it. */
export function CatalogCard(props: CatalogCardProps) {
  const accent = () => props.entry.accent ?? FALLBACK_ACCENT;
  const href = () => `/games/${props.entry.slug}`;
  const variantCount = () => props.variantCount ?? 0;
  const genre = () => (props.showKind === false ? undefined : genreTags(props.entry)[0]);

  return (
    <article
      class="game-card"
      classList={{ "game-card-lead": props.size === "lead", "game-card-side": props.size === "side" }}
      data-testid="catalog-card"
      data-slug={props.entry.slug}
      data-featured={String(props.entry.featured)}
      style={{ "--accent": accent() }}
    >
      <div class="game-card-picture">
        <Show
          when={showsBoard(props.entry.skin) ? props.entry.board : undefined}
          fallback={
            <div
              class="game-card-tint"
              style={{ background: `linear-gradient(135deg, ${accent()}, #1A0A00)` }}
            >
              <Show when={showcaseOf(props.entry.skin).length > 0}>
                <CardFan
                  skin={props.entry.skin}
                  codes={showcaseOf(props.entry.skin)}
                  players={props.entry.board?.players ?? []}
                />
              </Show>
            </div>
          }
        >
          {(board) => <GameBoard view={board()} skin={props.entry.skin} />}
        </Show>
      </div>
      <Dynamic component={`h${props.headingLevel ?? 3}`} class="game-card-title">
        <A href={href()} class="game-card-link">
          {props.entry.name}
        </A>
      </Dynamic>
      <p class="game-card-desc">{props.entry.tagline}</p>
      <Show when={props.size === "lead"}>
        <span class="btn-primary-lg" aria-hidden="true">
          Play now
        </span>
      </Show>
      <div class="game-card-meta">
        <Show when={genre()}>{(tag) => <span class="game-card-tag">{tag()}</span>}</Show>
        <Show when={variantCount() > 0}>
          <A href={`${href()}#variants`} class="game-card-variants">
            {variantCount()} {variantCount() === 1 ? "variant" : "variants"}
            <span aria-hidden="true"> →</span>
          </A>
        </Show>
      </div>
    </article>
  );
}
