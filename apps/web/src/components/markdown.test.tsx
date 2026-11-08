import { render } from "@solidjs/testing-library";
import { describe, expect, it } from "vitest";
import { Markdown } from "./markdown";

describe("Markdown", () => {
  it("renders paragraphs, bullet lists, bold, italics and code", () => {
    const { container } = render(() => (
      <Markdown source={"First *paragraph* with `code`.\n\n- **one**\n- two\n\nLast."} />
    ));

    expect(container.querySelectorAll("p")).toHaveLength(2);
    expect(container.querySelectorAll("li")).toHaveLength(2);
    expect(container.querySelector("li strong")?.textContent).toBe("one");
    expect(container.querySelector("em")?.textContent).toBe("paragraph");
    expect(container.querySelector("code")?.textContent).toBe("code");
  });

  it("shows markup as text instead of running it", () => {
    const { container } = render(() => (
      <Markdown source={'<img src=x onerror="alert(1)"> and <script>alert(2)</script>'} />
    ));

    expect(container.querySelector("img")).toBeNull();
    expect(container.querySelector("script")).toBeNull();
    expect(container.textContent).toContain("<img src=x");
  });
});
