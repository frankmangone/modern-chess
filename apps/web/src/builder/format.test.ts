import { describe, expect, it } from "vitest";
import { formatJson, LINE_WIDTH } from "./format";

/** Every game spec the engine ships, as text, by path. */
const specs = import.meta.glob("../../../../crates/engine/specs/*.json", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

describe("formatJson", () => {
  it("keeps what fits on one line on one line", () => {
    expect(formatJson({ board: { dimensions: [3, 5] } })).toBe('{ "board": { "dimensions": [3, 5] } }');
  });

  it("breaks what is too wide, and keeps the short parts of it together", () => {
    const board = {
      shape: "map",
      places: [
        { name: "top-left", at: [0, 2], capacity: 1 },
        { name: "top-right", at: [2, 2], capacity: 1 },
        { name: "centre", at: [1, 1], capacity: 1 },
      ],
    };
    expect(formatJson({ format_version: 1, name: "DO-GUTI", board })).toBe(
      [
        "{",
        '  "format_version": 1,',
        '  "name": "DO-GUTI",',
        '  "board": {',
        '    "shape": "map",',
        '    "places": [',
        '      { "name": "top-left", "at": [0, 2], "capacity": 1 },',
        '      { "name": "top-right", "at": [2, 2], "capacity": 1 },',
        '      { "name": "centre", "at": [1, 1], "capacity": 1 }',
        "    ]",
        "  }",
        "}",
      ].join("\n"),
    );
  });

  it("counts the indent, the key and the comma towards the width", () => {
    // `  "k": [1, 2],` is 14 columns with its comma: it stays whole at width 14 and breaks at 13.
    const value = { k: [1, 2], z: 1 };
    expect(formatJson(value, 14)).toBe('{\n  "k": [1, 2],\n  "z": 1\n}');
    expect(formatJson(value, 13)).toBe('{\n  "k": [\n    1, 2\n  ],\n  "z": 1\n}');
  });

  it("keeps a whole value on a line when it fits exactly", () => {
    // `{ "k": [1, 2] }` is 15 columns, and nothing follows it.
    expect(formatJson({ k: [1, 2] }, 16)).toBe('{ "k": [1, 2] }');
    expect(formatJson({ k: [1, 2] }, 15)).toBe('{\n  "k": [1, 2]\n}');
  });

  it("never lets a long list run off the line", () => {
    const text = formatJson({ first: 1, k: Array.from({ length: 40 }, (_, n) => 1000 + n) });
    for (const line of text.split("\n")) expect(line.length).toBeLessThanOrEqual(LINE_WIDTH);
    expect(text.split("\n").length).toBeLessThan(10);
  });

  it("packs a list of coordinates into rows, and a list of anything else one to a line", () => {
    const squares = Array.from({ length: 8 }, (_, n) => [n, 1]);
    expect(formatJson({ positions: { piece: "PAWN", at: squares } }, 40)).toBe(
      [
        "{",
        '  "positions": {',
        '    "piece": "PAWN",',
        '    "at": [',
        "      [0, 1], [1, 1], [2, 1], [3, 1],",
        "      [4, 1], [5, 1], [6, 1], [7, 1]",
        "    ]",
        "  }",
        "}",
      ].join("\n"),
    );
    const names = Array.from({ length: 8 }, (_, n) => `place-${n}`);
    // `{`, `"names": [`, eight names, `]` and `}`.
    expect(formatJson({ names }, 40).split("\n")).toHaveLength(12);
  });

  it("writes empty things as they are and escapes strings as JSON does", () => {
    expect(formatJson({ a: [], b: {}, c: 'say "hi"\n' })).toBe('{ "a": [], "b": {}, "c": "say \\"hi\\"\\n" }');
    expect(formatJson([])).toBe("[]");
    expect(formatJson(null)).toBe("null");
    expect(formatJson("é")).toBe('"é"');
  });

  it("breaks at a narrower width than the default when asked", () => {
    expect(formatJson({ at: [10, 20] }, 17)).toBe('{\n  "at": [10, 20]\n}');
    expect(formatJson({ at: [10, 20] }, 16)).toBe('{\n  "at": [\n    10, 20\n  ]\n}');
  });

  it("only changes whitespace: every real spec reads back as the same value", () => {
    const files = Object.entries(specs);
    expect(files.length).toBeGreaterThan(30);
    for (const [name, source] of files) {
      const value: unknown = JSON.parse(source);
      const text = formatJson(value);
      expect(JSON.parse(text), name).toEqual(value);
      // Nothing is wider than the width unless a single value cannot be broken (a long string).
      for (const line of text.split("\n")) {
        if (line.length > LINE_WIDTH) expect(/: "|^\s*"/.test(line), `${name}: ${line}`).toBe(true);
      }
    }
  });

  it("makes real specs much shorter than two-space JSON", () => {
    const source = Object.entries(specs).find(([name]) => name.endsWith("/do_guti.json"))?.[1] ?? "";
    const value: unknown = JSON.parse(source);
    expect(formatJson(value).split("\n").length).toBeLessThan(JSON.stringify(value, null, 2).split("\n").length / 2);
  });
});
