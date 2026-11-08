import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { snapshot } from "../test/fixtures";
import { startPolling } from "./poll";

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
});

describe("startPolling", () => {
  it("asks again after the interval, passing the revision it already has", async () => {
    const load = vi.fn().mockResolvedValueOnce(snapshot({ revision: 4 })).mockResolvedValue(null);
    const poller = startPolling({ load, onState: vi.fn(), onError: vi.fn(), intervalMs: 1500 });

    await vi.advanceTimersByTimeAsync(0);
    await vi.advanceTimersByTimeAsync(1500);

    expect(load.mock.calls.map(([since]) => since)).toEqual([undefined, 4]);
    poller.stop();
  });

  it("poll_stops_when_the_game_is_finished", async () => {
    const load = vi.fn().mockResolvedValue(snapshot({ status: "finished", outcome: { type: "draw" } }));
    const onState = vi.fn();
    startPolling({ load, onState, onError: vi.fn(), intervalMs: 1500 });

    await vi.advanceTimersByTimeAsync(10_000);

    expect(load).toHaveBeenCalledTimes(1);
    expect(onState).toHaveBeenCalledTimes(1);
  });

  it("keeps trying after a failed request and says when it recovers", async () => {
    const load = vi
      .fn()
      .mockRejectedValueOnce(new TypeError("offline"))
      .mockResolvedValue(snapshot());
    const onError = vi.fn();
    const poller = startPolling({ load, onState: vi.fn(), onError, intervalMs: 1500 });

    await vi.advanceTimersByTimeAsync(0);
    await vi.advanceTimersByTimeAsync(1500);

    expect(onError.mock.calls.map(([e]) => (e === null ? "recovered" : "failed"))).toEqual([
      "failed",
      "recovered",
    ]);
    poller.stop();
  });

  it("refresh asks right away instead of waiting for the interval", async () => {
    const load = vi.fn().mockResolvedValue(null);
    const poller = startPolling({ load, onState: vi.fn(), onError: vi.fn(), intervalMs: 1500 });
    await vi.advanceTimersByTimeAsync(0);

    poller.refresh();
    await vi.advanceTimersByTimeAsync(0);

    expect(load).toHaveBeenCalledTimes(2);
    poller.stop();
  });

  it("does nothing after stop", async () => {
    const load = vi.fn().mockResolvedValue(null);
    const poller = startPolling({ load, onState: vi.fn(), onError: vi.fn(), intervalMs: 1500 });
    await vi.advanceTimersByTimeAsync(0);

    poller.stop();
    await vi.advanceTimersByTimeAsync(10_000);

    expect(load).toHaveBeenCalledTimes(1);
  });
});
