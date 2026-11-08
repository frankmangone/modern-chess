import { cleanup } from "@solidjs/testing-library";
import { afterEach, vi } from "vitest";

// jsdom has no scrolling; the router asks for it after each navigation.
window.scrollTo = vi.fn();

afterEach(() => {
  cleanup();
  localStorage.clear();
});
