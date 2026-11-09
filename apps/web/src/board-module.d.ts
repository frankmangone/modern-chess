/// <reference types="vite/client" />

// The board element is shared with the docs and written in plain JavaScript.
declare module "@board/modern-chess-board.js";

// The skin helpers the board element is built on. The page uses them to draw pieces the same
// way outside the board, such as in a hand.
declare module "@board/board-model.js" {
  export interface BoardSkin {
    pieces: Record<string, unknown>;
    squares: { light: string; dark: string };
    zones: Record<string, string>;
  }
  export interface PieceFace {
    kind: "glyph" | "svg" | "fallback";
    value: string;
    colour?: string;
  }
  export function normalizeSkin(raw: unknown): BoardSkin;
  export function pieceFace(skin: BoardSkin, code: string, player: string, players: string[]): PieceFace;
}
