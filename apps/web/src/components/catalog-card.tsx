import { A } from "@solidjs/router";
import { For, Show } from "solid-js";
import type { CatalogEntry } from "../api/types";

export type CardVariant = "featured" | "small" | "wide";

interface CatalogCardProps {
  entry: CatalogEntry;
  variant: CardVariant;
}

const FALLBACK_ACCENT = "#8B4513";

/** One game on the front page. The game's own accent tints the block that has no cover image. */
export function CatalogCard(props: CatalogCardProps) {
  const accent = () => props.entry.accent ?? FALLBACK_ACCENT;
  const tint = () => `linear-gradient(135deg, ${accent()}, #1A0A00)`;
  const cover = () => props.entry.cover_image;

  const frame = (className: string) => (
    <Show
      when={cover()}
      fallback={<div class={className} style={{ background: tint() }} />}
    >
      {(src) => <img class={className} src={src()} alt={props.entry.name} />}
    </Show>
  );

  return (
    <A
      href={`/games/${props.entry.slug}`}
      class={rootClass(props.variant)}
      data-testid="catalog-card"
      data-slug={props.entry.slug}
      data-variant={props.variant}
      style={{ "--accent": accent() }}
    >
      <Show when={props.variant === "featured"}>
        <div class="featured-card-img" style={{ background: cover() ? undefined : tint() }}>
          <Show when={cover()}>
            {(src) => <img src={src()} alt={props.entry.name} />}
          </Show>
          <span class="featured-badge">🏆 FEATURED</span>
        </div>
        <div class="featured-card-info">
          <h3 class="featured-card-title">{props.entry.name}</h3>
          <p class="featured-card-desc">{props.entry.tagline}</p>
          <div class="featured-card-meta">
            <span class="featured-tags">
              <For each={props.entry.tags}>{(tag) => <span class="genre-tag">{tag}</span>}</For>
            </span>
            <span class="btn-play">Play now</span>
          </div>
        </div>
      </Show>

      <Show when={props.variant === "small"}>
        {frame(cover() ? "small-card-img" : "small-card-img-placeholder")}
        <div class="small-card-info">
          <span class="small-card-title">{props.entry.name}</span>
          <span class="small-card-desc">{props.entry.tagline}</span>
          <Show when={props.entry.tags[0]}>
            {(tag) => <span class="genre-tag">{tag()}</span>}
          </Show>
        </div>
      </Show>

      <Show when={props.variant === "wide"}>
        {frame(cover() ? "row2-card-img" : "row2-card-img-placeholder")}
        <div class="row2-card-info">
          <span class="row2-card-title" style={{ color: "white" }}>
            {props.entry.name}
          </span>
          <span class="row2-card-desc" style={{ color: "#A07A5A" }}>
            {props.entry.tagline}
          </span>
          <Show when={props.entry.tags[0]}>
            {(tag) => <span class="row2-card-badge">{tag()}</span>}
          </Show>
        </div>
      </Show>
    </A>
  );
}

function rootClass(variant: CardVariant): string {
  if (variant === "featured") return "featured-card";
  if (variant === "small") return "small-card";
  return "row2-card";
}
