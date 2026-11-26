// The engine, loaded in Node, as the only judge of whether a spec is legal. It is the same
// WebAssembly the web app and the book load, built by `make wasm` into apps/docs/theme/pkg.
import { existsSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const PKG = fileURLToPath(new URL("../../../apps/docs/theme/pkg/", import.meta.url));

type Engine = typeof import("../../../apps/docs/theme/pkg/modern_chess_wasm.d.ts");

let engine: Promise<Engine> | undefined;

function load(): Promise<Engine> {
  engine ??= (async () => {
    const wasm = `${PKG}modern_chess_wasm_bg.wasm`;
    if (!existsSync(wasm)) throw new Error("the engine is not built: run `make wasm` first");
    const loaded: Engine = await import(`${PKG}modern_chess_wasm.js`);
    loaded.initSync({ module: readFileSync(wasm) });
    return loaded;
  })();
  return engine;
}

/** Reads a spec as the engine does. Throws with the engine's own message when it refuses. */
export async function readSpec(label: string, spec: unknown): Promise<void> {
  const { Board } = await load();
  let board: InstanceType<typeof Board> | undefined;
  try {
    board = new Board(JSON.stringify(spec));
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`the engine refuses ${label}: ${message}`);
  } finally {
    board?.free();
  }
}
