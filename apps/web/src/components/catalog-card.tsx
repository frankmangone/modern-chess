import { A } from "@solidjs/router";
import { For, Show } from "solid-js";
import type { CatalogEntry } from "../api/types";

interface CatalogCardProps {
  entry: CatalogEntry;
}

const FALLBACK_ACCENT = "#8B4513";

/** One game on the front page. The game's own accent tints the image block when there is no
 *  cover image. */
export function CatalogCard(props: CatalogCardProps) {
  const accent = () => props.entry.accent ?? FALLBACK_ACCENT;
  const tint = () => `linear-gradient(135deg, ${accent()}, #1A0A00)`;
  const cover = () => props.entry.cover_image;

  return (
    <A
      href={`/games/${props.entry.slug}`}
      class="game-card"
      data-testid="catalog-card"
      data-slug={props.entry.slug}
      data-featured={String(props.entry.featured)}
      style={{ "--accent": accent() }}
    >
      <div class="game-card-img" style={{ background: cover() ? undefined : tint() }}>
        <Show when={cover()}>{(src) => <img src={src()} alt={props.entry.name} loading="lazy" />}</Show>
        <Show when={props.entry.featured}>
          <span class="featured-badge">🏆 FEATURED</span>
        </Show>
      </div>
      <div class="game-card-info">
        <h3 class="game-card-title">{props.entry.name}</h3>
        <p class="game-card-desc">{props.entry.tagline}</p>
        <div class="game-card-meta">
          <span class="featured-tags">
            <For each={props.entry.tags}>{(tag) => <span class="genre-tag">{tag}</span>}</For>
          </span>
          <span class="btn-play">Play now</span>
        </div>
      </div>
    </A>
  );
}
