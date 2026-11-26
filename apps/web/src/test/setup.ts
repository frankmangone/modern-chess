import { cleanup } from "@solidjs/testing-library";
import { afterEach, vi } from "vitest";

// A test file may ask for the node environment (`@vitest-environment node`): no window then.
if (typeof window !== "undefined") {
  // jsdom has no scrolling; the router asks for it after each navigation.
  window.scrollTo = vi.fn();
}

afterEach(() => {
  if (typeof window === "undefined") return;
  cleanup();
  localStorage.clear();
});
