import { render } from "@solidjs/testing-library";
import { afterEach, describe, expect, it, vi } from "vitest";
import { boardView } from "../test/fixtures";
import { BoardPreview } from "./board-preview";

vi.mock("./board-loader", () => ({ loadBoardElement: vi.fn().mockResolvedValue(undefined) }));

afterEach(() => {
  vi.useRealTimers();
});

/** The element as the engine leaves it: no view until it is up, then one that follows the game. */
function engine(element: Element) {
  const state = { ready: false, view: boardView() };
  Object.defineProperty(element, "view", {
    get() {
      if (!state.ready) throw new Error("the engine is not up yet");
      return state.view;
    },
  });
  return state;
}

describe("BoardPreview", () => {
  it("tells_the_page_the_board_after_a_change_once_the_engine_is_up", async () => {
    const seen = vi.fn();
    render(() => <BoardPreview rules={{}} skin={null} onView={seen} />);
    const element = document.querySelector("modern-chess-board") as Element;
    const state = engine(element);

    element.append(document.createElement("div"));
    await Promise.resolve();
    expect(seen).not.toHaveBeenCalled();

    state.ready = true;
    state.view = boardView({ turn: "BLACK" });
    element.append(document.createElement("div"));

    await vi.waitFor(() => expect(seen).toHaveBeenCalledWith(state.view));
  });

  it("asks_again_until_the_engine_is_up_when_nothing_changes", async () => {
    vi.useFakeTimers();
    const seen = vi.fn();
    render(() => <BoardPreview rules={{}} skin={null} onView={seen} />);
    const element = document.querySelector("modern-chess-board") as Element;
    const state = engine(element);

    await vi.advanceTimersByTimeAsync(450);
    expect(seen).not.toHaveBeenCalled();

    state.ready = true;
    await vi.advanceTimersByTimeAsync(150);

    expect(seen).toHaveBeenCalledWith(state.view);
  });

  it("watches_nothing_when_the_page_does_not_ask_for_the_view", () => {
    const observe = vi.spyOn(MutationObserver.prototype, "observe");
    render(() => <BoardPreview rules={{}} skin={null} />);

    expect(observe).not.toHaveBeenCalled();
    observe.mockRestore();
  });
});
