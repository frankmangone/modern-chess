import { render, screen } from "@solidjs/testing-library";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { PageSection } from "../api/types";
import { CHESS_SECTIONS, GO_SKIN_13, detail, goDetail } from "../test/fixtures";
import { loadBoardElement } from "./board-loader";
import type { LobbyActions } from "./lobby-list";
import { SectionRenderer } from "./section-renderer";

vi.mock("./board-loader", () => ({ loadBoardElement: vi.fn().mockResolvedValue(undefined) }));

const idle: LobbyActions = { create: vi.fn(), join: vi.fn(), open: vi.fn(), you: null, busy: false, error: null };

function sectionTypes(): (string | null)[] {
  return screen.getAllByTestId("section").map((node) => node.getAttribute("data-section-type"));
}

beforeEach(() => {
  vi.mocked(loadBoardElement).mockClear();
});

describe("SectionRenderer", () => {
  it("sections_render_in_the_order_the_page_defines", () => {
    const sections: PageSection[] = [
      { type: "open_lobbies" },
      { type: "how_to_play", markdown: "Rules." },
      { type: "preview" },
    ];

    render(() => <SectionRenderer detail={detail({ page: { ...detail().page, sections } })} lobbies={idle} />);
    expect(sectionTypes()).toEqual(["open_lobbies", "how_to_play", "preview"]);
  });

  it("an_unknown_section_type_is_skipped_not_fatal", () => {
    const sections = [
      CHESS_SECTIONS[0],
      { type: "video", url: "https://example.com/x" },
      CHESS_SECTIONS[2],
    ] as PageSection[];

    render(() => <SectionRenderer detail={detail({ page: { ...detail().page, sections } })} lobbies={idle} />);
    expect(sectionTypes()).toEqual(["how_to_play", "open_lobbies"]);
  });

  it("the preview hands the board the game's rules and skin, and loads the board once", () => {
    const game = detail();
    render(() => <SectionRenderer detail={game} lobbies={idle} />);

    const board = document.querySelector("modern-chess-board");
    expect(board).not.toBeNull();
    const inline = board?.querySelector('script[type="application/json"]')?.textContent;
    expect(JSON.parse(inline ?? "null")).toEqual(game.spec.rules);
    const skinUrl = board?.getAttribute("skin") ?? "";
    expect(JSON.parse(decodeURIComponent(skinUrl.replace("data:application/json,", "")))).toEqual(
      game.page.skin,
    );
    expect(loadBoardElement).toHaveBeenCalledTimes(1);
  });

  it("the preview works for a game with no skin", () => {
    const game = detail({ page: { ...detail().page, skin: null } });
    render(() => <SectionRenderer detail={game} lobbies={idle} />);

    expect(document.querySelector("modern-chess-board")?.hasAttribute("skin")).toBe(false);
  });

  it("the preview uses the preview variant's own skin when it has one", () => {
    const game = goDetail({ preview_variant: "go-13" });
    render(() => <SectionRenderer detail={game} lobbies={idle} />);

    const skinUrl = document.querySelector("modern-chess-board")?.getAttribute("skin") ?? "";
    expect(JSON.parse(decodeURIComponent(skinUrl.replace("data:application/json,", "")))).toEqual(
      GO_SKIN_13,
    );
  });
});
