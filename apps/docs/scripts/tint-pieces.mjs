// Tints the game's chess pieces for the docs.
//
// The web app draws Chess with the pieces in `apps/web/public/skins/walnut-chess/`: an outline,
// three tones of body (boxwood for White, chocolate for Black) and, on the rook, bishop and knight,
// a darker mark. A book chapter's made-up game should look like the real one but not pass for it,
// so each player colour the board uses gets a copy of the white set with every body tone replaced
// by a tone of that colour. A colour the script does not know is an error, not a leftover.
// Four colours are made, for a game of up to four players. Output goes to
// `src/boards/pieces/<colour>/<piece>.svg` (generated, git-ignored).
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

/** The colours the board gives players 1 to 4 (PLAYER_COLOURS in theme/board-model.js). */
export const TINTS = { red: "#d94f4f", blue: "#3f6fd9", green: "#3fa86b", gold: "#d9a13f" };

/** The white set's three body tones, lightest to darkest. The outline (#1c0e06) is kept. */
export const BODY_TONES = ["#f6e8cc", "#e8cfa3", "#d4ab76"];

/** The darker mark on the rook's band, the bishop's cut and the knight's eye. Not every piece has it. */
export const MARK_TONE = "#b98a5e";

/** The outline, kept as it is. */
export const OUTLINE = "#1c0e06";

export const PIECES = ["K", "Q", "R", "B", "N", "P"];

const channels = (hex) => [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16));
const toHex = (rgb) => `#${rgb.map((c) => Math.round(c).toString(16).padStart(2, "0")).join("")}`;

/** `hex` moved `amount` (0 to 1) of the way towards `target`. */
export function mix(hex, target, amount) {
  const from = channels(hex);
  const to = channels(target);
  return toHex(from.map((c, i) => c + (to[i] - c) * amount));
}

/** The tones of a colour for a piece: light, the colour itself, dark, and darker still for the mark. */
export function tones(colour) {
  return [mix(colour, "#ffffff", 0.2), colour, mix(colour, "#000000", 0.25), mix(colour, "#000000", 0.5)];
}

/** A white piece's drawing with its body drawn in `colour`. */
export function tint(svg, colour) {
  const to = tones(colour);
  const known = new Set([OUTLINE, ...BODY_TONES, MARK_TONE]);
  const strange = [...new Set((svg.match(/#[0-9a-fA-F]{3,6}\b/g) ?? []).map((c) => c.toLowerCase()))].filter(
    (c) => !known.has(c),
  );
  if (strange.length > 0) throw new Error(`the piece has colours the tint does not know (${strange.join(", ")}): the set changed`);
  for (const tone of BODY_TONES) {
    if (!svg.includes(tone)) throw new Error(`the piece has no ${tone}: the set changed`);
  }
  let out = svg;
  [...BODY_TONES, MARK_TONE].forEach((tone, i) => {
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
