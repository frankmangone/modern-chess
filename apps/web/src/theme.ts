// A game may draw its pages in colours of its own. Its page names up to five, and every other
// colour the stylesheet uses is worked out from them here, so a theme stays small and its shades
// agree. A page without a theme keeps the walnut values in styles.css exactly as they are.

import { createEffect, onCleanup } from "solid-js";
import type { PageTheme } from "./api/types";

/** The five walnut colours a theme replaces, as `:root` in styles.css sets them. */
export const WALNUT: Required<PageTheme> = {
  ground: "#3a2315",
  paper: "#f3e5cb",
  ink: "#24130a",
  accent: "#f2c66b",
  mark: "#c9623a",
};

const BLACK = "#000000";

/** Every colour variable a theme sets, by name without the leading dashes. With no theme, none. */
export function themeVariables(theme: PageTheme | null | undefined): Record<string, string> {
  if (!theme) return {};
  const { ground, paper, ink, accent, mark } = { ...WALNUT, ...theme };
  // The mixes are the ones the walnut values sit at, near enough.
  const inkSoft = mix(ink, paper, 0.2);
  return {
    ground,
    "ground-deep": mix(ground, BLACK, 0.25),
    "on-ground": paper,
    "on-ground-soft": mix(paper, ground, 0.22),
    paper,
    "paper-2": mix(paper, ink, 0.07),
    ink,
    "ink-soft": inkSoft,
    "ink-on-accent": mix(ink, accent, 0.12),
    line: mix(ink, BLACK, 0.25),
    shadow: mix(ink, BLACK, 0.5),
    ember: mark,
    caramel: mix(mark, paper, 0.35),
    honey: accent,
    "highlight-ember": mix(mark, BLACK, 0.45),
  };
}

/** Draws the whole site in the theme while the calling page is open, and puts walnut back after.
 *  The variables go on the root element because the ground is painted on the page body. */
export function useTheme(theme: () => PageTheme | null | undefined): void {
  createEffect(() => {
    const variables = themeVariables(theme());
    const root = document.documentElement;
    for (const [name, value] of Object.entries(variables)) root.style.setProperty(`--${name}`, value);
    onCleanup(() => {
      for (const name of Object.keys(variables)) root.style.removeProperty(`--${name}`);
    });
  });
}

/** `a` with `amount` of `b` mixed in, as `color-mix(in srgb, …)` does it. */
export function mix(a: string, b: string, amount: number): string {
  const [x, y] = [channels(a), channels(b)];
  return `#${x.map((v, i) => Math.round(v + (y[i] - v) * amount).toString(16).padStart(2, "0")).join("")}`;
}

/** The WCAG contrast ratio of two `#rrggbb` colours, from 1 to 21. */
export function contrast(a: string, b: string): number {
  const [la, lb] = [luminance(a), luminance(b)];
  return (Math.max(la, lb) + 0.05) / (Math.min(la, lb) + 0.05);
}

function luminance(colour: string): number {
  const [r, g, b] = channels(colour).map((v) => {
    const c = v / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

function channels(colour: string): number[] {
  return [1, 3, 5].map((i) => parseInt(colour.slice(i, i + 2), 16));
}
