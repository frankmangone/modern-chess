import { A } from "@solidjs/router";
import { For, Show } from "solid-js";
import { Dynamic } from "solid-js/web";
import type { CatalogEntry } from "../api/types";
import { GameBoard } from "./game-board";

interface CatalogCardProps {
  entry: CatalogEntry;
  /** The level of the game's name: one below whatever heads the list the card sits in. */
  headingLevel?: 2 | 3 | 4;
  /** How many games are variants of this one: a link to them on the game's page. */
  variantCount?: number;
}

const FALLBACK_ACCENT = "#8B4513";

/** One game, as a folder: its board as a game starts on the front, its accent on the tab. The
 *  name is the link, stretched over the whole card; the link to its variants sits above it. */
export function CatalogCard(props: CatalogCardProps) {
  const accent = () => props.entry.accent ?? FALLBACK_ACCENT;
  const href = () => `/games/${props.entry.slug}`;
  const variantCount = () => props.variantCount ?? 0;

  return (
    <article
      class="game-card"
      data-testid="catalog-card"
      data-slug={props.entry.slug}
      data-featured={String(props.entry.featured)}
      style={{ "--accent": accent() }}
    >
      <div class="game-card-picture">
        <Show
          when={props.entry.board}
          fallback={
            <div
              class="game-card-tint"
              style={{ background: `linear-gradient(135deg, ${accent()}, #1A0A00)` }}
            />
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
      <Show when={variantCount() > 0}>
        <A href={`${href()}#variants`} class="game-card-variants">
          {variantCount()} {variantCount() === 1 ? "variant" : "variants"}
          <span aria-hidden="true"> →</span>
        </A>
      </Show>
      <div class="game-card-meta">
        <span class="featured-tags">
          <Show when={props.entry.featured}>
            <span class="featured-badge">Featured</span>
          </Show>
          <For each={props.entry.tags}>{(tag) => <span class="genre-tag">{tag}</span>}</For>
        </span>
        <span class="btn-play" aria-hidden="true">
          Play now
        </span>
      </div>
    </article>
  );
}
