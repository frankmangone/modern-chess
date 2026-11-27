import { render, screen } from "@solidjs/testing-library";
import { describe, expect, it } from "vitest";
import { Stepper } from "./stepper";

const steps = [
  { id: "name", title: "Your name" },
  { id: "prefs", title: "Preferences" },
  { id: "done", title: "Ready" },
];

describe("Stepper", () => {
  it("marks the step the person is on, and only that one", () => {
    render(() => <Stepper steps={steps} current={1} label="Set-up" />);

    const items = screen.getAllByRole("listitem");
    expect(items.map((item) => item.getAttribute("aria-current"))).toEqual([null, "step", null]);
    expect(items.map((item) => item.getAttribute("data-state"))).toEqual(["done", "current", "ahead"]);
  });

  it("is a labelled list in order, and says which steps are done", () => {
    render(() => <Stepper steps={steps} current={2} label="Set-up" />);

    expect(screen.getByRole("list", { name: "Set-up" })).toBeTruthy();
    expect(screen.getAllByRole("listitem").map((item) => item.textContent)).toEqual([
      "Your name (done)",
      "Preferences (done)",
      "3Ready",
    ]);
  });

  it("numbers the steps that are not done", () => {
    render(() => <Stepper steps={steps} current={0} label="Set-up" />);

    expect(screen.getAllByRole("listitem").map((item) => item.textContent)).toEqual(["1Your name", "2Preferences", "3Ready"]);
  });
});
