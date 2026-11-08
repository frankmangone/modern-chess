let loading: Promise<unknown> | undefined;

/** Loads the board element the docs also use (it brings the game engine as WebAssembly).
 *  Loaded once, on the first page that shows a board. Build `apps/docs/build.sh` first: it
 *  produces the engine bundle this imports. */
export function loadBoardElement(): Promise<void> {
  loading ??= Promise.all([
    import("@board/modern-chess-board.js"),
    import("@board/modern-chess-board.css"),
  ]).catch((error: unknown) => {
    loading = undefined;
    throw error;
  });
  return loading.then(() => undefined);
}
