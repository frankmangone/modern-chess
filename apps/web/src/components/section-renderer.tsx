import { For, type JSX } from "solid-js";
import type { GameDetail, PageSection } from "../api/types";
import { BoardPreview } from "./board-preview";
import { LobbyList, type LobbyActions } from "./lobby-list";
import { Markdown } from "./markdown";

interface SectionRendererProps {
  detail: GameDetail;
  lobbies: LobbyActions;
}

function Frame(props: { type: string; children: JSX.Element }) {
  return (
    <section class="game-section" data-testid="section" data-section-type={props.type}>
      {props.children}
    </section>
  );
}

/** Draws the page's sections in the order the page lists them. A type this app does not know
 *  is skipped, so a page written for a newer app still shows what this one can. */
export function SectionRenderer(props: SectionRendererProps) {
  /** "Try it" runs the preview variant: its own skin when it has one, else the game's. */
  const previewSkin = () =>
    props.detail.variants.find((v) => v.slug === props.detail.preview_variant)?.skin ??
    props.detail.page.skin;

  const view = (section: PageSection): JSX.Element => {
    switch (section.type) {
      case "how_to_play":
        return (
          <Frame type="how_to_play">
            <h2 class="section-title">How to play</h2>
            <Markdown source={section.markdown} />
          </Frame>
        );
      case "preview":
        return (
          <Frame type="preview">
            <h2 class="section-title">Try it</h2>
            <BoardPreview rules={props.detail.spec.rules} skin={previewSkin()} />
          </Frame>
        );
      case "open_lobbies":
        return (
          <Frame type="open_lobbies">
            <LobbyList
              lobbies={props.detail.open_lobbies}
              variants={props.detail.variants}
              preview={props.detail.preview_variant}
              actions={props.lobbies}
            />
          </Frame>
        );
      default:
        return null;
    }
  };

  return <For each={props.detail.page.sections}>{view}</For>;
}
