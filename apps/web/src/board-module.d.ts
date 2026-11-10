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
    facing: boolean;
    board: {
      style: "squares" | "intersections";
      background: string | null;
      lines: string | null;
      breaks: [number, number][];
      diagonals: [[number, number], [number, number]][];
      dots: [number, number][];
      crosses: [number, number][];
      labels: { text: string; x: number; y: number }[];
    };
    colours: Record<string, string>;
    pieceStyle: "plain" | "disc";
  }
  export interface PieceFace {
    kind: "glyph" | "svg" | "image" | "fallback";
    value: string;
    colour?: string;
    /** Set when the skin draws text and drawings on a round tile. */
    disc?: true;
  }
  export function normalizeSkin(raw: unknown): BoardSkin;
  export function isTurned(skin: BoardSkin, player: string, players: string[], flipped: boolean): boolean;
  export function pieceFace(skin: BoardSkin, code: string, player: string, players: string[]): PieceFace;
}
