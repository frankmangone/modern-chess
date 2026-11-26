import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { BODY_TONES, PIECES, TINTS, mix, tint, tones } from "./tint-pieces.mjs";

const white = `<svg><path style="fill:#1c0e06"/><path style="fill:${BODY_TONES[0]}"/><path style="fill:${BODY_TONES[1]}"/><path style="fill:${BODY_TONES[2]}"/></svg>`;

test("mix moves a colour towards another", () => {
  assert.equal(mix("#000000", "#ffffff", 0.5), "#808080");
  assert.equal(mix("#102030", "#ffffff", 0), "#102030");
  assert.equal(mix("#102030", "#ffffff", 1), "#ffffff");
});

test("the tones are a lighter, the colour itself and a darker", () => {
  const [light, middle, dark] = tones("#3f6fd9");
  assert.equal(middle, "#3f6fd9");
  assert.ok(parseInt(light.slice(1, 3), 16) > 0x3f);
  assert.ok(parseInt(dark.slice(1, 3), 16) < 0x3f);
});

test("tint replaces the three body tones and keeps the outline", () => {
  const out = tint(white, "#d94f4f");
  assert.ok(out.includes("#1c0e06"));
  for (const tone of BODY_TONES) assert.ok(!out.includes(tone));
  assert.ok(out.includes("#d94f4f"));
});

test("tint refuses a piece that is not drawn in the white set's tones", () => {
  assert.throws(() => tint('<svg><path style="fill:#1c0e06"/></svg>', "#d94f4f"), /the set changed/);
});

test("the game's own white pieces all take every tint", () => {
  for (const piece of PIECES) {
    const svg = readFileSync(new URL(`../../web/public/skins/walnut-chess/w${piece}.svg`, import.meta.url), "utf8");
    for (const colour of Object.values(TINTS)) assert.doesNotThrow(() => tint(svg, colour));
  }
});
