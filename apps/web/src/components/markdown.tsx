import { For, type JSX } from "solid-js";

// The few marks the game pages use: paragraphs, "- " lists, **bold**, *italic*, `code`.
// Everything is built as elements and text, never as HTML, so page text cannot run anything.

const INLINE = /\*\*(.+?)\*\*|\*(.+?)\*|`(.+?)`/g;

function inline(text: string): JSX.Element[] {
  const parts: JSX.Element[] = [];
  let last = 0;
  for (const match of text.matchAll(INLINE)) {
    if (match.index > last) parts.push(text.slice(last, match.index));
    const [, bold, italic, code] = match;
    if (bold !== undefined) parts.push(<strong>{inline(bold)}</strong>);
    else if (italic !== undefined) parts.push(<em>{inline(italic)}</em>);
    else if (code !== undefined) parts.push(<code>{code}</code>);
    last = match.index + match[0].length;
  }
  if (last < text.length) parts.push(text.slice(last));
  return parts;
}

/** A block is a list when every line in it starts with "- ". */
function isList(lines: string[]): boolean {
  return lines.every((line) => line.startsWith("- "));
}

export function Markdown(props: { source: string }) {
  const blocks = () =>
    props.source
      .split(/\n\s*\n/)
      .map((block) => block.split("\n").filter((line) => line.trim() !== ""))
      .filter((lines) => lines.length > 0);

  return (
    <div class="markdown">
      <For each={blocks()}>
        {(lines) =>
          isList(lines) ? (
            <ul>
              <For each={lines}>{(line) => <li>{inline(line.slice(2))}</li>}</For>
            </ul>
          ) : (
            <p>{inline(lines.join(" "))}</p>
          )
        }
      </For>
    </div>
  );
}
