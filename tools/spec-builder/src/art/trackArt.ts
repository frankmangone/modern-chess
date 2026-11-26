// The drawn board of a numbered track: the squares, the pad pieces wait on, ladders as two rails
// with rungs, snakes as curves with a head. Everything is written into a box `rowLength + 2`
// wide and `rows + 1` tall, in which square `n` at column c, row r sits at x = c + 1.5,
// y = rows - 0.5 - r (y runs down).
import { num } from "../num.ts";
import { squareAt, type Jumps } from "../recipes/track.ts";

export interface TrackArt {
  squares: number;
  rowLength: number;
  ladders: Jumps;
  snakes: Jumps;
  /** The colours snakes take in turn, in order of where they start. */
  snakeColours: string[];
}

export function trackArt(options: TrackArt): string {
  const { squares, rowLength, ladders, snakes, snakeColours } = options;
  const rows = Math.ceil(squares / rowLength);
  const centre = (n: number): [number, number] => {
    const [column, row] = squareAt(n, rowLength);
    return [column + 2, rows - row];
  };
  const parts: string[] = [`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${rowLength + 2} ${rows + 1}" preserveAspectRatio="none">`];
  // The pad pieces wait on before square 1.
  parts.push(`<rect x="0.55" y="${num(rows - 0.45)}" width="0.9" height="0.9" rx="0.12" fill="#cfe3b4" stroke="#6f8f4a" stroke-width="0.04"/>`);
  parts.push(`<text x="1" y="${num(rows + 0.08)}" font-size="0.2" font-family="sans-serif" font-weight="700" text-anchor="middle" fill="#4a6a2c">START</text>`);
  for (let n = 1; n <= squares; n += 1) {
    const [column, row] = squareAt(n, rowLength);
    const x = column + 1.5;
    const y = rows - 0.5 - row;
    const fill = (column + row) % 2 === 0 ? "#f4e7c6" : "#e4cf9f";
    parts.push(`<rect x="${num(x)}" y="${num(y)}" width="1" height="1" fill="${fill}" stroke="#b79a62" stroke-width="0.02"/>`);
    parts.push(`<text x="${num(x + 0.06)}" y="${num(y + 0.24)}" font-size="0.22" font-family="sans-serif" font-weight="600" fill="#8a6d3b">${n}</text>`);
  }
  // Kept as the saved art has it: it draws nothing.
  parts.push('<rect x="1.5" y="-0.5" width="0" height="0"/>');
  for (const [from, to] of ladders) {
    const [x1, y1] = centre(from);
    const [x2, y2] = centre(to);
    const dx = x2 - x1;
    const dy = y2 - y1;
    const length = Math.hypot(dx, dy);
    const ux = dx / length;
    const uy = dy / length;
    const px = -uy * 0.13;
    const py = ux * 0.13;
    parts.push('<g stroke="#8a5a2b" stroke-linecap="round" opacity="0.92">');
    parts.push(`<line x1="${num(x1 + px)}" y1="${num(y1 + py)}" x2="${num(x2 + px)}" y2="${num(y2 + py)}" stroke-width="0.07"/>`);
    parts.push(`<line x1="${num(x1 - px)}" y1="${num(y1 - py)}" x2="${num(x2 - px)}" y2="${num(y2 - py)}" stroke-width="0.07"/>`);
    const rungs = Math.max(2, Math.trunc(length / 0.42));
    for (let i = 1; i < rungs; i += 1) {
      const t = i / rungs;
      const cx = x1 + dx * t;
      const cy = y1 + dy * t;
      parts.push(`<line x1="${num(cx + px)}" y1="${num(cy + py)}" x2="${num(cx - px)}" y2="${num(cy - py)}" stroke-width="0.05"/>`);
    }
    parts.push("</g>");
  }
  // A snake's head is where it starts, its tail where it lands.
  [...snakes]
    .sort(([a], [b]) => a - b)
    .forEach(([from, to], i) => {
      const [x1, y1] = centre(from);
      const [x2, y2] = centre(to);
      const dx = x2 - x1;
      const dy = y2 - y1;
      const length = Math.hypot(dx, dy);
      const nx = -dy / length;
      const ny = dx / length;
      const bend = Math.min(1.0, (0.35 * length) / 3 + 0.2);
      const c1: [number, number] = [x1 + dx * 0.3 + nx * bend, y1 + dy * 0.3 + ny * bend];
      const c2: [number, number] = [x1 + dx * 0.7 - nx * bend, y1 + dy * 0.7 - ny * bend];
      const colour = snakeColours[i % snakeColours.length];
      parts.push(`<path d="M${num(x1)} ${num(y1)} C${num(c1[0])} ${num(c1[1])} ${num(c2[0])} ${num(c2[1])} ${num(x2)} ${num(y2)}" fill="none" stroke="${colour}" stroke-width="0.17" stroke-linecap="round" opacity="0.9"/>`);
      parts.push(`<circle cx="${num(x1)}" cy="${num(y1)}" r="0.16" fill="${colour}"/>`);
      parts.push(`<circle cx="${num(x1 - 0.05)}" cy="${num(y1 - 0.04)}" r="0.035" fill="#fff"/><circle cx="${num(x1 + 0.05)}" cy="${num(y1 - 0.04)}" r="0.035" fill="#fff"/>`);
    });
  parts.push("</svg>");
  return parts.join("");
}
