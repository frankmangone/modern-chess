import { render } from "@solidjs/testing-library";
import { createSignal } from "solid-js";
import { describe, expect, it } from "vitest";
import { usePageTitle } from "./page-title";

describe("usePageTitle", () => {
  it("puts_the_page_name_before_the_site_name", () => {
    render(() => {
      usePageTitle(() => "Chess");
      return null;
    });

    expect(document.title).toBe("Chess · BoardHub");
  });

  it("is_just_the_site_name_when_the_page_has_no_name_yet", () => {
    render(() => {
      usePageTitle(() => undefined);
      return null;
    });

    expect(document.title).toBe("BoardHub");
  });

  it("follows_the_name_as_it_changes", () => {
    const [name, setName] = createSignal<string | undefined>(undefined);
    render(() => {
      usePageTitle(name);
      return null;
    });

    setName("Go");

    expect(document.title).toBe("Go · BoardHub");
  });
});
