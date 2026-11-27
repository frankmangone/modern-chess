import { fireEvent, render, screen } from "@solidjs/testing-library";
import { afterEach, describe, expect, it, vi } from "vitest";
import { InviteButton } from "./invite-button";

const URL = "http://localhost/play/abc";

function pointer(coarse: boolean) {
  vi.stubGlobal("matchMedia", (query: string) => ({ matches: coarse && query === "(pointer: coarse)" }));
}

describe("InviteButton", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
    Reflect.deleteProperty(navigator, "share");
  });

  it("copies_the_link_on_a_computer_and_says_so", async () => {
    pointer(false);
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });
    render(() => <InviteButton url={URL} name="Chess" />);

    fireEvent.click(screen.getByRole("button", { name: "Invite a friend" }));

    expect(await screen.findByRole("button", { name: "Link copied" })).toBeTruthy();
    expect(writeText).toHaveBeenCalledWith(URL);
  });

  it("opens_the_share_sheet_on_a_phone", async () => {
    pointer(true);
    const share = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "share", { value: share, configurable: true });
    render(() => <InviteButton url={URL} name="Chess" />);

    fireEvent.click(screen.getByRole("button", { name: "Invite a friend" }));

    await vi.waitFor(() => expect(share).toHaveBeenCalledWith(expect.objectContaining({ url: URL })));
  });
});
