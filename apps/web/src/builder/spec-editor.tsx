import { For, createMemo, createSignal } from "solid-js";

/** One line of the editor, in pixels: the numbers and the text must agree on it. */
const LINE = 20;
const PAD = 12;

export interface SpecEditorHandle {
  /** Puts the cursor on a line (counting from 1) and selects it, scrolled into view. */
  goTo: (line: number) => void;
}

interface SpecEditorProps {
  label: string;
  value: string;
  onInput: (text: string) => void;
  /** The line to mark as the one at fault. */
  flag?: number | null;
  placeholder?: string;
  onRun?: () => void;
  ref?: (handle: SpecEditorHandle) => void;
}

/** A plain text box for a spec, with line numbers and a mark on the line the engine complains
 *  about. It is deliberately not a code editor: the visual editor will replace most of the typing. */
export function SpecEditor(props: SpecEditorProps) {
  let text: HTMLTextAreaElement | undefined;
  let gutter: HTMLDivElement | undefined;
  const [scroll, setScroll] = createSignal(0);
  const lines = createMemo(() => props.value.split("\n").length);

  props.ref?.({
    goTo(line) {
      if (!text) return;
      const rows = props.value.split("\n");
      const at = Math.min(Math.max(line, 1), rows.length);
      const start = rows.slice(0, at - 1).reduce((sum, row) => sum + row.length + 1, 0);
      text.focus();
      text.setSelectionRange(start, start + rows[at - 1].length);
      text.scrollTop = Math.max(0, (at - 1) * LINE - text.clientHeight / 2);
    },
  });

  function synchronise() {
    if (!text) return;
    setScroll(text.scrollTop);
    if (gutter) gutter.scrollTop = text.scrollTop;
  }

  return (
    <div class="editor">
      <div class="editor-gutter" ref={gutter} aria-hidden="true">
        <For each={Array.from({ length: lines() }, (_, i) => i + 1)}>{(n) => <div>{n}</div>}</For>
      </div>
      {props.flag ? (
        <div class="editor-flag" style={{ top: `${PAD + (props.flag - 1) * LINE - scroll()}px` }} aria-hidden="true" />
      ) : null}
      <textarea
        ref={text}
        class="editor-text"
        aria-label={props.label}
        value={props.value}
        placeholder={props.placeholder}
        spellcheck={false}
        autocapitalize="off"
        autocomplete="off"
        wrap="off"
        onInput={(event) => props.onInput(event.currentTarget.value)}
        onScroll={synchronise}
        onKeyDown={(event) => {
          if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
            event.preventDefault();
            props.onRun?.();
          }
        }}
      />
    </div>
  );
}
