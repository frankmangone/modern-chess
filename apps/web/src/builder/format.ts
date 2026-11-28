/** How wide a line of JSON may be before it is broken into several. */
export const LINE_WIDTH = 80;

/**
 * JSON written to be read: anything that fits on one line within the width stays on one line, and
 * what does not is broken one item per line, two spaces deeper. The exception is a list of numbers
 * or of coordinates, which is packed into as few lines as it takes (a row of squares, not a column).
 * Only whitespace differs from `JSON.stringify`, so reading it back gives the same value.
 *
 *     "board": { "dimensions": [3, 5] }
 */
export function formatJson(value: unknown, width: number = LINE_WIDTH): string {
  return lay(value, 0, 0, width);
}

type Container = unknown[] | { [key: string]: unknown };

function isContainer(value: unknown): value is Container {
  return typeof value === "object" && value !== null;
}

/** A number, or a list of them, however deep: what a coordinate or a list of coordinates is made of. */
function isNumeric(value: unknown): boolean {
  return typeof value === "number" || (Array.isArray(value) && value.every(isNumeric));
}

/** `value` on one line: `[3, 5]`, `{ "at": [0, 2], "capacity": 1 }`. */
function flat(value: unknown): string {
  if (!isContainer(value)) return JSON.stringify(value);
  if (Array.isArray(value)) return value.length === 0 ? "[]" : `[${value.map(flat).join(", ")}]`;
  const entries = Object.entries(value);
  if (entries.length === 0) return "{}";
  return `{ ${entries.map(([key, item]) => `${JSON.stringify(key)}: ${flat(item)}`).join(", ")} }`;
}

/**
 * `value` where it starts at column `used` of its line (its indent, and its key if it has one). A
 * comma may follow it, so one more column is kept free.
 */
function lay(value: unknown, indent: number, used: number, width: number): string {
  if (!isContainer(value)) return JSON.stringify(value);
  const line = flat(value);
  if (used + line.length + 1 <= width || line === "[]" || line === "{}") return line;

  const inner = indent + 2;
  const pad = " ".repeat(inner);
  const close = " ".repeat(indent);
  if (Array.isArray(value) && value.every(isNumeric)) {
    const rows: string[] = [];
    let row = "";
    for (const item of value) {
      const text = flat(item);
      // The comma after an item stays inside the width too.
      if (row !== "" && pad.length + row.length + 2 + text.length + 1 > width) {
        rows.push(`${pad}${row},`);
        row = text;
      } else {
        row = row === "" ? text : `${row}, ${text}`;
      }
    }
    rows.push(`${pad}${row}`);
    return `[\n${rows.join("\n")}\n${close}]`;
  }
  if (Array.isArray(value)) {
    const items = value.map((item) => `${pad}${lay(item, inner, inner, width)}`);
    return `[\n${items.join(",\n")}\n${close}]`;
  }
  const items = Object.entries(value).map(([key, item]) => {
    const prefix = `${JSON.stringify(key)}: `;
    return `${pad}${prefix}${lay(item, inner, inner + prefix.length, width)}`;
  });
  return `{\n${items.join(",\n")}\n${close}}`;
}
