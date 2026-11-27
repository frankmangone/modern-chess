/** A guard against a game that hangs the page. The key is set while a game is being started and
 *  cleared once its board has drawn, so a reload does not start the same game again and leaves a way
 *  to edit it. Storage can be shut; without it there is no guard and nothing else depends on it. */

export function createRunGuard(key: string) {
  return {
    /** Whether a run of `id` did not finish drawing last time. */
    holds(id: string | null): boolean {
      try {
        return id !== null && localStorage.getItem(key) === id;
      } catch {
        return false;
      }
    },
    /** Sets the guard for `id`, or clears it with null. */
    set(id: string | null): void {
      try {
        if (id === null) localStorage.removeItem(key);
        else localStorage.setItem(key, id);
      } catch {
        // See above.
      }
    },
  };
}
