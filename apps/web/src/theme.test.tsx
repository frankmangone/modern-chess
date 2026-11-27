import { render } from "@solidjs/testing-library";
import { createSignal, Show } from "solid-js";
import { describe, expect, it } from "vitest";
import type { PageTheme } from "./api/types";
import { WALNUT, contrast, mix, themeVariables, useTheme } from "./theme";

// The stylesheet as text, for its walnut values.
const styles = Object.values(import.meta.glob<string>("./styles.css", { eager: true, query: "?inline", import: "default" }))[0];

// The seeded game pages: a theme on any of them must keep every text readable.
const PAGES = import.meta.glob<{ slug: string; theme?: PageTheme; skin?: { squares?: { light: string; dark: string } } }>(
  "../../../crates/game/pages/*.json",
  { eager: true, import: "default" },
);

/** The pairs the stylesheet draws text or marks in, and the contrast each needs. */
function problems(theme: PageTheme, squares?: { light: string; dark: string }): string[] {
  const v = themeVariables(theme);
  const pairs: [string, string, string, number][] = [
    ["ink", "paper", "text on paper", 7],
    ["ink-soft", "paper", "quiet text on paper", 7],
    ["ink-soft", "paper-2", "quiet text on the second paper", 6],
    ["on-ground", "ground", "text on the ground", 7],
    ["on-ground-soft", "ground", "quiet text on the ground", 4.5],
    ["honey", "ground", "accent text on the ground", 4.5],
    ["ink", "honey", "text on the accent", 7],
    ["ink-on-accent", "honey", "quiet text on the accent", 4.5],
  ];
  const found = pairs
    .filter(([a, b, , least]) => contrast(v[a], v[b]) < least)
    .map(([a, b, what, least]) => `${what} (${a} on ${b}): ${contrast(v[a], v[b]).toFixed(2)} < ${least}`);
  for (const square of squares ? [squares.light, squares.dark] : []) {
    if (contrast(v["highlight-ember"], square) < 3) found.push(`board highlight on ${square}`);
  }
  return found;
}

describe("themeVariables", () => {
  it("sets_nothing_without_a_theme", () => {
    expect(themeVariables(null)).toEqual({});
  });

  it("keeps_walnut_for_each_colour_a_theme_leaves_out", () => {
    const variables = themeVariables({ ground: "#333333" });

    expect(variables.ground).toBe("#333333");
    expect(variables.paper).toBe(WALNUT.paper);
    expect(variables.honey).toBe(WALNUT.accent);
  });

  it("starts_from_the_walnut_in_the_stylesheet", () => {
    const root = (name: string) => styles.match(new RegExp(`--${name}: (#[0-9a-f]{6});`))?.[1];

    expect(WALNUT).toEqual({
      ground: root("ground"),
      paper: root("paper"),
      ink: root("ink"),
      accent: root("honey"),
      mark: root("ember"),
    });
  });

  it("derives_shades_close_to_the_walnut_ones", () => {
    const variables = themeVariables(WALNUT);
    for (const name of ["ground-deep", "paper-2", "ink-soft", "line", "shadow", "highlight-ember"]) {
      const walnut = styles.match(new RegExp(`--${name}: (#[0-9a-f]{6});`))![1];
      expect(contrast(variables[name], walnut), name).toBeLessThan(1.25);
    }
  });

  it("mixes_like_color_mix_in_srgb", () => {
    expect(mix("#000000", "#ffffff", 0.5)).toBe("#808080");
    expect(mix("#3a2315", "#000000", 0)).toBe("#3a2315");
  });
});

describe("seeded themes", () => {
  it("the_walnut_colours_pass_the_same_checks", () => {
    expect(problems(WALNUT)).toEqual([]);
  });

  it("keep_every_text_readable", () => {
    const themed = Object.values(PAGES).filter((page) => page.theme);

    expect(themed.map((page) => page.slug)).toContain("modern-chess");
    for (const page of themed) {
      expect(problems(page.theme!, page.skin?.squares), page.slug).toEqual([]);
    }
  });
});

describe("useTheme", () => {
  const ground = () => document.documentElement.style.getPropertyValue("--ground");

  it("draws_the_site_in_the_theme_while_the_page_is_open_and_puts_walnut_back_after", () => {
    const [open, setOpen] = createSignal(true);
    function Page() {
      useTheme(() => ({ ground: "#34322e" }));
      return null;
    }
    render(() => <Show when={open()}><Page /></Show>);

    expect(ground()).toBe("#34322e");
    setOpen(false);
    expect(ground()).toBe("");
  });

  it("follows_the_theme_once_the_game_loads", () => {
    const [theme, setTheme] = createSignal<PageTheme | null>(null);
    render(() => {
      useTheme(theme);
      return null;
    });

    expect(ground()).toBe("");
    setTheme({ ground: "#34322e" });
    expect(ground()).toBe("#34322e");
  });
});
