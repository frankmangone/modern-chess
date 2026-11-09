/** The dot that stands for each seat, by turn order: the first seat is light and the second
 *  dark, as on the board. More seats than that get colours of their own. */
const SEAT_COLOURS = ["#F0E8DE", "#2A2A2A", "#FF6B35", "#4A6AFF"];

export function seatColour(players: string[], seat: string): string {
  const index = Math.max(0, players.indexOf(seat));
  return SEAT_COLOURS[index % SEAT_COLOURS.length] ?? SEAT_COLOURS[0]!;
}
