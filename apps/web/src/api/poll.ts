import type { InstanceState } from "./types";

export const POLL_INTERVAL_MS = 1500;

interface PollOptions {
  /** The match if it changed since revision `since`, or null if it did not. */
  load: (since: number | undefined) => Promise<InstanceState | null>;
  onState: (state: InstanceState) => void;
  /** Called with the failure, or with null once a request works again. */
  onError: (error: unknown) => void;
  intervalMs?: number;
}

export interface Poller {
  stop: () => void;
  /** Asks right now instead of waiting for the next turn. */
  refresh: () => void;
  /** Tells the poller about a newer state it did not fetch itself, such as a command's reply. */
  accept: (state: InstanceState) => void;
}

/** Keeps a match fresh. It stops by itself once the match is finished, and carries on through
 *  failed requests, since a dropped connection should not end a game. */
export function startPolling(options: PollOptions): Poller {
  const interval = options.intervalMs ?? POLL_INTERVAL_MS;
  let since: number | undefined;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let stopped = false;
  let asking = false;

  const stop = () => {
    stopped = true;
    clearTimeout(timer);
  };

  async function ask() {
    if (stopped || asking) return;
    asking = true;
    try {
      const state = await options.load(since);
      if (stopped) return;
      options.onError(null);
      // An answer that was already on its way when a newer state arrived (a command's reply)
      // would put the board back a step.
      const late = state !== null && since !== undefined && state.revision < since;
      if (state !== null && !late) {
        since = state.revision;
        options.onState(state);
        if (state.status === "finished") stop();
      }
    } catch (error) {
      if (!stopped) options.onError(error);
    } finally {
      asking = false;
    }
    if (!stopped) timer = setTimeout(() => void ask(), interval);
  }

  void ask();

  return {
    stop,
    refresh: () => {
      clearTimeout(timer);
      void ask();
    },
    accept: (state) => {
      since = state.revision;
      if (state.status === "finished") stop();
    },
  };
}
