// The drawn board of a backgammon table: a walnut frame, two dark wooden halves with twenty-four
// triangles, the bar between them and a tray for each side. A box 15 wide and 9 tall in which a place at map
// [x, y] sits at (x + 1, 8 - y).
import { num } from "../num.ts";
import { pointAt } from "../recipes/backgammon.ts";

export const FRAME = "#4a2c14";
const WOOD = "#3a2413";
const BAR = "#6b4426";
const TRIANGLE_LIGHT = "#e0c58a";
const TRIANGLE_DARK = "#a24b2c";

export function backgammonArt(): string {
  const parts: string[] = ['<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 15 9" preserveAspectRatio="none">'];
  parts.push(`<rect x="0" y="0" width="15" height="9" fill="${FRAME}"/>`);
  parts.push(`<rect x="0.5" y="0.5" width="6" height="8" fill="${WOOD}"/>`);
  parts.push(`<rect x="7.5" y="0.5" width="6" height="8" fill="${WOOD}"/>`);
  parts.push(`<rect x="6.5" y="0.5" width="1" height="8" fill="${BAR}"/>`);
  parts.push('<text x="7" y="4.62" font-size="0.3" font-family="sans-serif" font-weight="700" text-anchor="middle" fill="#d9bf8f" letter-spacing="0.08">BAR</text>');
  // The trays: Black's above, White's below.
  for (const y of [0.55, 7.55]) {
    parts.push(`<rect x="13.55" y="${y}" width="0.9" height="0.9" rx="0.1" fill="#3a2410" stroke="#d9bf8f" stroke-width="0.03"/>`);
  }
  parts.push('<text x="14" y="1.78" font-size="0.2" font-family="sans-serif" font-weight="700" text-anchor="middle" fill="#d9bf8f">BLACK</text>');
  parts.push('<text x="14" y="7.4" font-size="0.2" font-family="sans-serif" font-weight="700" text-anchor="middle" fill="#d9bf8f">WHITE</text>');
  for (let p = 1; p <= 24; p += 1) {
    const [column] = pointAt(p);
    const cx = column + 1;
    const left = cx - 0.5;
    const top = p >= 13;
    const fill = (p % 2 === 0) === top ? TRIANGLE_LIGHT : TRIANGLE_DARK;
    const points = top ? `${num(left)},0.5 ${num(left + 1)},0.5 ${num(cx)},4` : `${num(left)},8.5 ${num(left + 1)},8.5 ${num(cx)},5`;
    const numberY = top ? 0.36 : 8.82;
    parts.push(`<polygon points="${points}" fill="${fill}" stroke="#00000033" stroke-width="0.02"/>`);
    parts.push(`<text x="${num(cx)}" y="${numberY}" font-size="0.26" font-family="sans-serif" font-weight="600" text-anchor="middle" fill="#d9bf8f">${p}</text>`);
  }
  parts.push("</svg>");
  return parts.join("");
}
