// Tints the game's chess pieces for the docs.
//
// The web app draws Chess with the pieces in `apps/web/public/skins/walnut-chess/`: an outline
// and three tones of body, boxwood for White and chocolate for Black. A book chapter's made-up
// game should look like the real one but not pass for it, so each player colour the board
// uses gets a copy of the white set with the three body tones replaced by tones of that colour.
// Four colours are made, for a game of up to four players. Output goes to
// `src/boards/pieces/<colour>/<piece>.svg` (generated, git-ignored).
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

/** The colours the board gives players 1 to 4 (PLAYER_COLOURS in theme/board-model.js). */
export const TINTS = { red: "#d94f4f", blue: "#3f6fd9", green: "#3fa86b", gold: "#d9a13f" };

/** The white set's three body tones, lightest to darkest. The outline (#1c0e06) is kept. */
export const BODY_TONES = ["#f6e8cc", "#e8cfa3", "#d4ab76"];

export const PIECES = ["K", "Q", "R", "B", "N", "P"];

const channels = (hex) => [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16));
const toHex = (rgb) => `#${rgb.map((c) => Math.round(c).toString(16).padStart(2, "0")).join("")}`;

/** `hex` moved `amount` (0 to 1) of the way towards `target`. */
export function mix(hex, target, amount) {
  const from = channels(hex);
  const to = channels(target);
  return toHex(from.map((c, i) => c + (to[i] - c) * amount));
}

/** The three tones of a colour for a piece's body: light, the colour itself, dark. */
export function tones(colour) {
  return [mix(colour, "#ffffff", 0.2), colour, mix(colour, "#000000", 0.25)];
}

/** A white piece's drawing with its body drawn in `colour`. */
export function tint(svg, colour) {
  const to = tones(colour);
  let out = svg;
  BODY_TONES.forEach((tone, i) => {
    if (!out.includes(tone)) throw new Error(`the piece has no ${tone}: the set changed`);
    out = out.replaceAll(tone, to[i]);
  });
  return out;
}

function main() {
  const here = dirname(fileURLToPath(import.meta.url));
  const docs = join(here, "..");
  const from = join(docs, "..", "web", "public", "skins", "walnut-chess");
  for (const [name, colour] of Object.entries(TINTS)) {
    const dir = join(docs, "src", "boards", "pieces", name);
    mkdirSync(dir, { recursive: true });
    for (const piece of PIECES) {
      writeFileSync(join(dir, `${piece}.svg`), tint(readFileSync(join(from, `w${piece}.svg`), "utf8"), colour));
    }
    console.log(`tinted ${PIECES.length} pieces ${name}`);
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) main();
