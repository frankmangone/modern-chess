import { describe, expect, it } from "vitest";
import { waited } from "./lobby-list";

const at = (iso: string) => Date.parse(iso);

describe("waited", () => {
  const created = "2026-11-08T10:00:00Z";

  it("says_just_now_for_under_a_minute_and_for_a_clock_that_is_behind", () => {
    expect(waited(created, at("2026-11-08T10:00:30Z"))).toBe("just now");
    expect(waited(created, at("2026-11-08T09:59:00Z"))).toBe("just now");
  });

  it("counts_minutes_hours_and_days_in_the_biggest_unit_that_fits", () => {
    expect(waited(created, at("2026-11-08T10:06:00Z"))).toBe("6 min");
    expect(waited(created, at("2026-11-08T10:59:00Z"))).toBe("59 min");
    expect(waited(created, at("2026-11-08T13:10:00Z"))).toBe("3 h");
    expect(waited(created, at("2026-11-10T11:00:00Z"))).toBe("2 d");
  });

  it("does_not_make_up_a_time_it_cannot_read", () => {
    expect(waited("not a date", at("2026-11-08T10:00:00Z"))).toBe("just now");
  });
});
