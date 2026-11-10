import { afterEach, describe, expect, it, vi } from "vitest";
import "@board/modern-chess-board.js";
import { boardView } from "../test/fixtures";
import type { BoardView } from "../api/types";

interface BoardElement extends HTMLElement {
  view: BoardView | null;
  interactive: boolean;
  dropCode: string | null;
  externalPrompts: boolean;
}

async function mount(view: BoardView, interactive = true): Promise<BoardElement> {
  const board = document.createElement("modern-chess-board") as BoardElement;
  board.setAttribute("remote", "");
  document.body.append(board);
  board.view = view;
  board.interactive = interactive;
  await Promise.resolve();
  await Promise.resolve();
  return board;
}

function cell(label: string): HTMLButtonElement {
  const found = document.querySelector<HTMLButtonElement>(`[aria-label="${label}"]`);
  if (found === null) throw new Error(`no cell labelled ${label}`);
  return found;
}

afterEach(() => {
  document.body.replaceChildren();
  vi.restoreAllMocks();
});

describe("<modern-chess-board remote>", () => {
  it("remote_mode_renders_a_given_view_without_loading_wasm", async () => {
    const fetched = vi.spyOn(globalThis, "fetch");
    const instantiate = vi.spyOn(WebAssembly, "instantiate");

    const board = await mount(boardView({ turn: "BLACK" }));

    expect(board.querySelectorAll(".mcb-cell")).toHaveLength(9);
    expect(cell("WHITE ROOK at 0,0")).toBeTruthy();
    expect(board.querySelector(".mcb-status")?.textContent).toContain("BLACK to move");
    expect(board.querySelector(".mcb-reset")).toBeNull();
    expect(fetched).not.toHaveBeenCalled();
    expect(instantiate).not.toHaveBeenCalled();
  });

  it("shows a new view when the property is set again", async () => {
    const board = await mount(boardView());

    board.view = boardView({ pieces: [{ x: 1, y: 1, code: "ROOK", player: "WHITE" }] });

    expect(board.querySelectorAll(".mcb-piece")).toHaveLength(1);
    expect(cell("WHITE ROOK at 1,1")).toBeTruthy();
  });

  it("remote_mode_emits_select_and_move_events", async () => {
    const board = await mount(boardView({ selected: [0, 0], destinations: [[0, 1]] }));
    const heard: [string, unknown][] = [];
    for (const type of ["select", "move", "promote"]) {
      board.addEventListener(type, (event) => heard.push([type, (event as CustomEvent).detail]));
    }

    cell("WHITE ROOK at 0,0").click();
    cell("empty square 0,1").click();

    expect(heard).toEqual([
      ["select", { x: 0, y: 0 }],
      ["move", { x: 0, y: 1 }],
    ]);
  });

  it("emits promote with the chosen piece", async () => {
    const board = await mount(boardView({ promotion: ["QUEEN", "ROOK"] }));
    const heard: unknown[] = [];
    board.addEventListener("promote", (event) => heard.push((event as CustomEvent).detail));

    board.querySelector<HTMLButtonElement>('.mcb-promo-option[title="QUEEN"]')?.click();

    expect(heard).toEqual([{ code: "QUEEN" }]);
  });

  it("does not let a click through when it is not the viewer's turn", async () => {
    const board = await mount(boardView(), false);
    const heard: Event[] = [];
    board.addEventListener("select", (event) => heard.push(event));

    expect(cell("WHITE ROOK at 0,0").disabled).toBe(true);
    cell("WHITE ROOK at 0,0").click();

    expect(heard).toHaveLength(0);
  });

  it("takes properties set before the element was defined", async () => {
    const early = document.createElement("div");
    early.innerHTML = "<modern-chess-board remote></modern-chess-board>";
    const board = early.firstElementChild as BoardElement;
    expect(board.view).toBeNull();

    document.body.append(early);
    board.view = boardView();
    await Promise.resolve();
    await Promise.resolve();

    expect(board.querySelectorAll(".mcb-cell")).toHaveLength(9);
  });

  it("takes a skin that arrives after the board is already showing", async () => {
    const board = await mount(boardView());
    const skin = { squares: { light: "#111111", dark: "#222222" } };

    board.setAttribute("skin", `data:application/json,${encodeURIComponent(JSON.stringify(skin))}`);
    await vi.waitFor(() => expect(board.style.getPropertyValue("--mcb-light")).toBe("#111111"));

    expect(board.querySelectorAll(".mcb-cell")).toHaveLength(9);
  });

  it("turns_the_board_around_for_the_second_player", async () => {
    const board = await mount(boardView());
    const order = () => Array.from(board.querySelectorAll(".mcb-cell")).map((c) => c.getAttribute("aria-label"));
    expect(order()[0]).toBe("empty square 0,2");

    (board as BoardElement & { flipped: boolean }).flipped = true;

    expect(order()[0]).toBe("empty square 2,0");
    expect(order().at(-1)).toBe("empty square 0,2");
  });

  it("shows_no_promotion_choices_to_someone_who_cannot_make_them", async () => {
    const board = await mount(boardView({ promotion: ["QUEEN", "ROOK"] }), false);

    expect(board.querySelectorAll(".mcb-promo-option")).toHaveLength(0);
    expect(board.querySelector(".mcb-status")?.textContent).toContain("choose a promotion");
  });

  it("leaves_the_promotion_choice_to_the_page_when_asked", async () => {
    const board = await mount(boardView({ promotion: ["QUEEN", "ROOK"] }));
    expect(board.querySelectorAll(".mcb-promo-option")).toHaveLength(2);

    board.externalPrompts = true;

    expect(board.querySelectorAll(".mcb-promo-option")).toHaveLength(0);
  });
});

describe("<modern-chess-board remote> with a hand", () => {
  const handGame = (overrides: Partial<BoardView> = {}) =>
    boardView({
      hands: true,
      hand: { WHITE: { PAWN: 1 }, BLACK: {} },
      drops: { PAWN: [[1, 1], [1, 2]] },
      ...overrides,
    });

  function heardFrom(board: BoardElement): [string, unknown][] {
    const heard: [string, unknown][] = [];
    for (const type of ["select", "move", "dropPiece"]) {
      board.addEventListener(type, (event) => heard.push([type, (event as CustomEvent).detail]));
    }
    return heard;
  }

  it("a_hand_game_can_be_played_on_a_remote_board", async () => {
    const board = await mount(handGame());

    expect(cell("WHITE ROOK at 0,0").disabled).toBe(false);
    expect(board.querySelector(".mcb-note")).toBeNull();
  });

  it("shows_where_the_chosen_hand_piece_may_go_and_emits_a_drop", async () => {
    const board = await mount(handGame());
    const heard = heardFrom(board);

    board.dropCode = "PAWN";

    // Rows are drawn top first, so the higher square comes first.
    expect(Array.from(board.querySelectorAll(".mcb-dest")).map((c) => c.getAttribute("aria-label"))).toEqual([
      "empty square 1,2",
      "empty square 1,1",
    ]);
    cell("empty square 1,1").click();
    cell("empty square 2,1").click();

    expect(heard).toEqual([["dropPiece", { code: "PAWN", x: 1, y: 1 }]]);
  });

  it("shows_the_piece_destinations_again_when_no_hand_piece_is_chosen", async () => {
    const board = await mount(handGame({ selected: [0, 0], destinations: [[0, 1]] }));
    board.dropCode = "PAWN";
    expect(board.querySelectorAll(".mcb-dest")).toHaveLength(2);

    board.dropCode = null;

    expect(Array.from(board.querySelectorAll(".mcb-dest")).map((c) => c.getAttribute("aria-label"))).toEqual([
      "empty square 0,1",
    ]);
  });

  it("a_chosen_hand_piece_with_no_squares_offers_nothing", async () => {
    const board = await mount(handGame({ drops: { PAWN: [] } }));
    const heard = heardFrom(board);

    board.dropCode = "PAWN";
    cell("empty square 1,1").click();

    expect(board.querySelectorAll(".mcb-dest")).toHaveLength(0);
    expect(heard).toEqual([]);
  });

  it("a_click_on_your_own_piece_still_selects_it_while_a_hand_piece_is_chosen", async () => {
    const board = await mount(handGame());
    const heard = heardFrom(board);
    board.dropCode = "PAWN";

    cell("WHITE ROOK at 0,0").click();

    expect(heard).toEqual([["select", { x: 0, y: 0 }]]);
  });
});

describe("<modern-chess-board remote> with a picture skin", () => {
  const skinUrl = (skin: object) => `data:application/json,${encodeURIComponent(JSON.stringify(skin))}`;
  const pictures = {
    facing: true,
    pieces: {
      ROOK: {
        WHITE: { image: "/skins/test/wR.svg" },
        BLACK: { image: "/skins/test/bR.svg" },
      },
    },
  };

  async function mountWithSkin(skin: object, flipped = false): Promise<BoardElement> {
    const board = await mount(boardView());
    (board as BoardElement & { flipped: boolean }).flipped = flipped;
    board.setAttribute("skin", skinUrl(skin));
    await vi.waitFor(() => expect(board.querySelector("img.mcb-img")).not.toBeNull());
    return board;
  }

  it("draws_a_picture_face_as_an_image_with_no_text", async () => {
    const board = await mountWithSkin(pictures);

    const images = Array.from(board.querySelectorAll<HTMLImageElement>(".mcb-cell img.mcb-img"));
    expect(images.map((i) => i.getAttribute("src"))).toEqual(["/skins/test/bR.svg", "/skins/test/wR.svg"]);
    expect(images.every((i) => i.alt === "")).toBe(true);
    expect(cell("WHITE ROOK at 0,0").getAttribute("aria-label")).toBe("WHITE ROOK at 0,0");
  });

  it("turns_the_pieces_of_the_player_at_the_top_when_the_skin_asks_for_it", async () => {
    const board = await mountWithSkin(pictures);

    const turned = (label: string) => cell(label).querySelector(".mcb-piece")?.classList.contains("mcb-turned");
    expect(turned("WHITE ROOK at 0,0")).toBe(false);
    expect(turned("BLACK ROOK at 2,2")).toBe(true);
  });

  it("turns_the_other_side_when_the_board_is_flipped", async () => {
    const board = await mountWithSkin(pictures, true);

    const turned = (label: string) => cell(label).querySelector(".mcb-piece")?.classList.contains("mcb-turned");
    expect(turned("WHITE ROOK at 0,0")).toBe(true);
    expect(turned("BLACK ROOK at 2,2")).toBe(false);
  });

  it("leaves_pieces_upright_without_the_facing_option", async () => {
    const board = await mountWithSkin({ pieces: pictures.pieces });

    expect(board.querySelectorAll(".mcb-turned")).toHaveLength(0);
  });

  it("keeps_the_promotion_choices_upright_for_the_player_at_the_top", async () => {
    const board = await mount(boardView({ turn: "BLACK", promotion: ["ROOK"] }));
    board.setAttribute("skin", skinUrl(pictures));
    await vi.waitFor(() => expect(board.querySelector("img.mcb-img")).not.toBeNull());

    expect(board.querySelector(".mcb-promo-option .mcb-piece")?.classList.contains("mcb-turned")).toBe(false);
  });
});


describe("<modern-chess-board remote> with an intersection board", () => {
  const skinUrl = (skin: object) => `data:application/json,${encodeURIComponent(JSON.stringify(skin))}`;
  const lined = {
    board: {
      style: "intersections",
      background: "#e8c98a",
      lines: "#5c3b1e",
      // The lines between the bottom row and the one above it stop, but for the two edge files.
      breaks: [[0, 1]],
      diagonals: [[[0, 0], [2, 2]]],
      dots: [[0, 0]],
      crosses: [[0, 2]],
      labels: [{ text: "RIVER", x: 1, y: 0.5 }],
    },
  };

  async function mountLined(skin: object, flipped = false): Promise<BoardElement> {
    const board = await mount(boardView());
    (board as BoardElement & { flipped: boolean }).flipped = flipped;
    board.setAttribute("skin", skinUrl(skin));
    await vi.waitFor(() => expect(board.querySelector("svg.mcb-lines")).not.toBeNull());
    return board;
  }

  it("draws_the_lines_under_the_squares_so_pieces_sit_on_the_crossings", async () => {
    const board = await mountLined(lined);

    const grid = board.querySelector(".mcb-grid");
    expect(grid?.classList.contains("mcb-intersections")).toBe(true);
    expect((grid as HTMLElement).style.getPropertyValue("--mcb-board")).toBe("#e8c98a");
    expect(grid?.firstElementChild?.tagName.toLowerCase()).toBe("svg");
    // 3 rows across, the 2 edge files in one piece, the middle file only above the break, 1 diagonal.
    expect(board.querySelectorAll("svg.mcb-lines line")).toHaveLength(3 + 2 + 1 + 1);
    expect(board.querySelectorAll("svg.mcb-lines circle")).toHaveLength(1);
    expect(board.querySelectorAll("svg.mcb-lines path.mcb-cross")).toHaveLength(1);
    expect(board.querySelector("svg.mcb-lines text")?.textContent).toBe("RIVER");
  });

  it("leaves_the_svg_out_of_the_way_of_clicks_and_the_screen_reader", async () => {
    const board = await mountLined(lined);

    const svg = board.querySelector("svg.mcb-lines");
    expect(svg?.getAttribute("aria-hidden")).toBe("true");
    expect(svg?.getAttribute("viewBox")).toBe("0 0 3 3");
  });

  it("puts_a_mark_where_the_board_shows_that_crossing", async () => {
    const board = await mountLined(lined);
    const flipped = await mountLined(lined, true);

    // (0,0) is the bottom left of the first player's board, and the top right when it is flipped.
    expect(board.querySelector("svg.mcb-lines circle")?.getAttribute("cx")).toBe("0.5");
    expect(board.querySelector("svg.mcb-lines circle")?.getAttribute("cy")).toBe("2.5");
    expect(flipped.querySelector("svg.mcb-lines circle")?.getAttribute("cx")).toBe("2.5");
    expect(flipped.querySelector("svg.mcb-lines circle")?.getAttribute("cy")).toBe("0.5");
  });

  it("keeps_the_square_board_for_a_skin_that_does_not_ask_for_lines", async () => {
    const board = await mount(boardView());

    expect(board.querySelector("svg.mcb-lines")).toBeNull();
    expect(board.querySelector(".mcb-grid")?.classList.contains("mcb-intersections")).toBe(false);
  });

  it("draws_text_pieces_on_round_tiles_in_the_players_colour_when_the_skin_asks_for_it", async () => {
    const board = await mount(boardView());
    board.setAttribute(
      "skin",
      skinUrl({ pieceStyle: "disc", colours: { WHITE: "#c0392b" }, pieces: { ROOK: { WHITE: "R", BLACK: "r" } } }),
    );
    await vi.waitFor(() => expect(board.querySelector(".mcb-disc")).not.toBeNull());

    const white = board.querySelector<HTMLElement>('[aria-label="WHITE ROOK at 0,0"] .mcb-piece');
    expect(white?.classList.contains("mcb-disc")).toBe(true);
    expect(white?.style.color).toBe("rgb(192, 57, 43)");
    expect(white?.style.getPropertyValue("--mcb-ring")).toBe("#c0392b");
  });
});
