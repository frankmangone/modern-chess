// Dice thrown into a bag.
import { remove, roll, type BagRef, type Effect } from "../blocks.ts";

/** Faces of a die that each add one kind of piece, as many as the number it shows: a throw of 4
 *  puts four `code` pieces in the bag, and the bag's count is the number thrown. */
export function countFaces(code: string, sides = 6): Record<string, number>[] {
  return Array.from({ length: sides }, (_, k) => ({ [code]: k + 1 }));
}

/** Faces that each add one piece of their own: face 4 puts a `prefix` + "4" piece in the bag. */
export function codeFaces(prefix: string, sides = 6): Record<string, number>[] {
  return Array.from({ length: sides }, (_, k) => ({ [`${prefix}${k + 1}`]: 1 }));
}

/** Empties the bag, then throws into it. */
export function throwIntoBag(bag: BagRef, faces: Record<string, number>[], dice?: number): Effect[] {
  return [remove(bag, "ALL"), roll(bag, faces, dice)];
}
