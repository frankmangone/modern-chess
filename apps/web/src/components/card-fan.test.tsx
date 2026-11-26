import { render } from "@solidjs/testing-library";
import { describe, expect, it } from "vitest";
import { CardFan, showcaseOf } from "./card-fan";

describe("showcaseOf", () => {
  it("lists the codes a skin shows off, and nothing when it has none", () => {
    expect(showcaseOf({ showcase: ["C01", "C13"] })).toEqual(["C01", "C13"]);
    expect(showcaseOf({})).toEqual([]);
    expect(showcaseOf(null)).toEqual([]);
  });

  it("ignores what is not a list of text", () => {
    expect(showcaseOf({ showcase: "C01" })).toEqual([]);
    expect(showcaseOf({ showcase: ["C01", 7, null] })).toEqual(["C01"]);
  });
});

describe("CardFan", () => {
  const skin = { pieces: { C01: { svg: "<svg><title>one</title></svg>" } } };

  it("draws one card for each code, hidden from assistive technology", () => {
    const { container } = render(() => <CardFan skin={skin} codes={["C01", "C02", "C03"]} players={["RED", "BLUE"]} />);

    const fan = container.querySelector(".card-fan") as HTMLElement;
    expect(fan.getAttribute("aria-hidden")).toBe("true");
    expect(fan.querySelectorAll(".card-fan-card").length).toBe(3);
  });

  it("tilts the cards from one side to the other around the middle", () => {
    const { container } = render(() => <CardFan skin={skin} codes={["C01", "C02", "C03"]} players={["RED", "BLUE"]} />);

    const turns = [...container.querySelectorAll<HTMLElement>(".card-fan-card")].map((card) => card.style.transform);
    expect(turns[0]).toContain("rotate(-12deg)");
    expect(turns[1]).toContain("rotate(0deg)");
    expect(turns[2]).toContain("rotate(12deg)");
  });

  it("copes with a game that has no players listed", () => {
    const { container } = render(() => <CardFan skin={skin} codes={["C01"]} players={[]} />);

    expect(container.querySelectorAll(".card-fan-card").length).toBe(1);
  });
});
