/** What the engine says about a spec's text (see `check_spec` in crates/wasm). A refusal says where
 *  the text is at fault when it can: a rule that contradicts another part of the spec has no place. */
export interface SpecProblem {
  message: string;
  /** Counting from 1. */
  line: number | null;
  column: number | null;
  /** The keys that lead to it: `board.wide`, `pieces[2].moves[0]`. */
  path: string | null;
}

export type SpecCheck = ({ ok: true; name: string; players: string[] }) | ({ ok: false } & SpecProblem);

interface EngineModule {
  default: () => Promise<unknown>;
  check_spec: (specJson: string) => string;
}

let loading: Promise<EngineModule> | undefined;

/** The engine as the board element loads it: the same WebAssembly module, started once. */
function loadEngine(): Promise<EngineModule> {
  loading ??= import("@board/pkg/modern_chess_wasm.js")
    .then(async (engine: EngineModule) => {
      await engine.default();
      return engine;
    })
    .catch((error: unknown) => {
      loading = undefined;
      throw error;
    });
  return loading;
}

/** Reads `text` the way a game would be opened. */
export async function checkSpec(text: string): Promise<SpecCheck> {
  const engine = await loadEngine();
  return JSON.parse(engine.check_spec(text)) as SpecCheck;
}
