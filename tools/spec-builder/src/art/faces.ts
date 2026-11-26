// Piece faces: small drawings a skin gives to pieces, in a 100 by 100 box.

/** A round checker with a ring and a shine. */
export function disc(fill: string, edge: string, shine: string): string {
  return (
    `<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg"><circle cx="50" cy="50" r="44" fill="${fill}" stroke="${edge}" stroke-width="7"/>` +
    `<circle cx="50" cy="50" r="30" fill="none" stroke="${edge}" stroke-width="3" opacity="0.5"/>` +
    `<ellipse cx="36" cy="34" rx="13" ry="7" fill="${shine}"/></svg>`
  );
}

const PIPS: Record<number, [number, number][]> = {
  1: [[50, 50]],
  2: [[28, 28], [72, 72]],
  3: [[28, 28], [50, 50], [72, 72]],
  4: [[28, 28], [72, 28], [28, 72], [72, 72]],
  5: [[28, 28], [72, 28], [50, 50], [28, 72], [72, 72]],
  6: [[28, 26], [72, 26], [28, 50], [72, 50], [28, 74], [72, 74]],
};

/** A die showing `n`, from 1 to 6. */
export function die(n: number): string {
  const pips = PIPS[n];
  if (pips === undefined) throw new Error(`die: there is no face ${n}`);
  const dots = pips.map(([x, y]) => `<circle cx="${x}" cy="${y}" r="9" fill="#3b2a14"/>`).join("");
  return (
    '<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg"><rect x="8" y="8" width="84" height="84" rx="16" ' +
    `fill="#fbf6ea" stroke="#5a4326" stroke-width="6"/>${dots}</svg>`
  );
}
