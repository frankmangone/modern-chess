import { For } from "solid-js";

/** Squares along one side. */
const SIZE = 7;

export interface HeroCell {
  /** The checker colour: light or dark. */
  dark: boolean;
  /** No square here: the board has been cut away. */
  hole: boolean;
  /** Part of the zone, the far row a rule can point at. */
  zone: boolean;
}

/** A board drawn the way the game builder will let a person draw one: the corners and the middle
 *  cut away, and the far row marked as a zone. Row 0 is the top row. */
export function heroCells(): HeroCell[] {
  const last = SIZE - 1;
  const middle = Math.floor(SIZE / 2);
  const cells: HeroCell[] = [];
  for (let row = 0; row < SIZE; row += 1) {
    for (let column = 0; column < SIZE; column += 1) {
      const corner = (row === 0 || row === last) && (column === 0 || column === last);
      const centre = row === middle && column === middle;
      const hole = corner || centre;
      cells.push({ dark: (row + column) % 2 === 1, hole, zone: row === 0 && !hole });
    }
  }
  return cells;
}

const SPARKLE = "M22 3c1 10 4 17 19 19-15 2-18 9-19 19-1-10-4-17-19-19 15-2 18-9 19-19z";

/** The picture beside the headline: a builder-style board on a tilted sticker. It carries no
 *  meaning of its own, so it is hidden from assistive technology. */
export function HeroBoard() {
  return (
    <div class="hero-board-wrap" data-testid="hero-board" aria-hidden="true">
      <svg class="hero-sparkle hero-sparkle-a" viewBox="0 0 44 44">
        <path d={SPARKLE} />
      </svg>
      <svg class="hero-sparkle hero-sparkle-b" viewBox="0 0 44 44">
        <path d={SPARKLE} />
      </svg>
      <div class="hero-sticker">
        <div class="hero-board" style={{ "--n": SIZE }}>
          <For each={heroCells()}>
            {(cell) => (
              <span
                class="hero-cell"
                classList={{ dark: cell.dark, hole: cell.hole, zone: cell.zone }}
              />
            )}
          </For>
        </div>
      </div>
    </div>
  );
}
