/**
 * Whether the game has a board to draw. A game whose pieces live in hands and piles (a card
 * game) has no use for the square the board element would draw, so its skin turns it off with
 * `"board_panel": false`: the match page leaves it out, the try-it preview gives way to the
 * game's picture, and the catalogue shows the accent block.
 */
export function showsBoard(skin: Record<string, unknown> | null | undefined): boolean {
  return skin?.board_panel !== false;
}
