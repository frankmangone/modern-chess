import { type Accessor, type Setter, createEffect, createSignal, on, onCleanup } from "solid-js";
import { gameApi } from "../api/game";
import { ApiError } from "../api/http";
import { withAccount } from "../auth/identity";
import { type Draft, type Drafts, newDraft } from "./drafts";
import { type Remote, type SyncEvent, reconcile, step, withoutAccountDrafts } from "./sync";

/** How long after the last change the account is written. The browser's copy is written sooner. */
const SAVE_DELAY_MS = 800;
/** How long after a failure a save is tried again. */
const RETRY_DELAY_MS = 15_000;

interface Options {
  store: Accessor<Drafts>;
  setStore: Setter<Drafts>;
  /** Writes the drafts to the browser. */
  persist: () => void;
  /** Whether a signed-in account is known, and the sign-in has finished. */
  signedIn: Accessor<boolean>;
  /** Whether it is known yet if anyone is signed in: false while the sign-in is still being finished. */
  settled: Accessor<boolean>;
  /** The drafts to start from when none are left. */
  fresh: () => Drafts;
}

function fromRecord(record: Remote): Draft {
  return {
    ...newDraft(record.name, record.rules, record.skin, { kind: "synced", version: record.version }),
    remote: record.id,
    shared: record.share_token,
  };
}

/** Whether a draft is the starter nobody has changed: it is not worth a place beside real drafts. */
function untouchedStarter(draft: Draft, starter: string): boolean {
  return draft.sync.kind === "local" && draft.remote === undefined && draft.rules === starter && draft.skin === "";
}

/** Keeps a signed-in person's drafts on their account, as `sync.ts` says. The page keeps the text; this
 *  keeps the account's copy up to date, picks up what other browsers saved, and answers share requests.
 *  Everything it does to a draft goes through `step`, so a late answer cannot put a draft in a state it
 *  does not belong in. */
export function createAccountSync(options: Options, starter: string) {
  const { store, setStore, persist } = options;
  const [loaded, setLoaded] = createSignal(false);
  const [notice, setNotice] = createSignal<string | null>(null);
  const timers = new Map<string, ReturnType<typeof setTimeout>>();
  onCleanup(() => {
    for (const timer of timers.values()) clearTimeout(timer);
  });

  const find = (id: string) => store().drafts.find((d) => d.id === id);

  /** Changes one draft (and not its `updated`: nothing here is the person editing). */
  function patch(id: string, change: (draft: Draft) => Draft) {
    setStore((now) => ({ ...now, drafts: now.drafts.map((d) => (d.id === id ? change(d) : d)) }));
    persist();
  }
  const move = (id: string, event: SyncEvent) => patch(id, (d) => ({ ...d, sync: step(d.sync, event) }));

  // ---- Picking the account's drafts up when someone is signed in, and putting them down when not ----
  createEffect(
    on(
      () => [options.settled(), options.signedIn()] as const,
      ([settled, signedIn]) => {
        if (!settled) return;
        if (!signedIn) {
          setLoaded(false);
          if (store().drafts.some((d) => d.remote !== undefined)) {
            const rest = withoutAccountDrafts(store().drafts);
            setStore(rest.length > 0 ? { drafts: rest, current: rest[0].id } : options.fresh());
            persist();
          }
          return;
        }
        void load();
      },
    ),
  );

  async function load() {
    try {
      const records = await withAccount((token) => gameApi.listDrafts(token));
      const merged = reconcile(store().drafts, records, fromRecord);
      // The starter nobody touched is not kept beside real drafts.
      const drafts = records.length > 0 ? merged.filter((d) => !untouchedStarter(d, starter)) : merged;
      const current = drafts.some((d) => d.id === store().current) ? store().current : (drafts[0]?.id ?? null);
      setStore(drafts.length > 0 ? { drafts, current } : options.fresh());
      persist();
      setLoaded(true);
    } catch {
      setNotice("We could not reach your account to load your drafts. They are kept in this browser, and your changes are kept for later.");
      // Without the list, saving would guess at versions; it waits for the next visit.
    }
  }

  // ---- Saving ---------------------------------------------------------------------------------
  createEffect(() => {
    if (!options.signedIn() || !loaded()) return;
    for (const draft of store().drafts) {
      if (draft.sync.kind === "dirty") {
        clearTimeout(timers.get(draft.id));
        timers.set(draft.id, setTimeout(() => void send(draft.id), SAVE_DELAY_MS));
      } else if (draft.sync.kind === "failed") {
        clearTimeout(timers.get(draft.id));
        timers.set(draft.id, setTimeout(() => move(draft.id, { type: "retry" }), RETRY_DELAY_MS));
      } else {
        clearTimeout(timers.get(draft.id));
        timers.delete(draft.id);
      }
    }
  });

  async function send(id: string) {
    const draft = find(id);
    if (!draft || draft.sync.kind !== "dirty") return;
    const based = draft.sync.version;
    const body = { name: draft.name, rules: draft.rules, skin: draft.skin };
    move(id, { type: "send" });
    try {
      const record =
        draft.remote === undefined || based === null
          ? await withAccount((token) => gameApi.createDraft(body, token))
          : await withAccount((token) => gameApi.saveDraft(draft.remote!, { ...body, version: based }, token));
      if (find(id) === undefined) {
        // The person deleted the draft while it was being created: do not leave a copy behind.
        void withAccount((token) => gameApi.deleteDraft(record.id, token)).catch(() => undefined);
        return;
      }
      patch(id, (d) => ({
        ...d,
        remote: record.id,
        shared: record.share_token,
        sync: step(d.sync, { type: "saved", version: record.version }),
      }));
    } catch (error) {
      if (find(id) === undefined) return;
      if (error instanceof ApiError && error.status === 409 && error.currentVersion !== undefined) {
        move(id, { type: "conflict", current: error.currentVersion });
      } else if (error instanceof ApiError && error.status === 404 && draft.remote !== undefined) {
        patch(id, (d) => ({ ...d, remote: undefined, shared: null, sync: step(d.sync, { type: "gone" }) }));
        setNotice(`“${draft.name}” was deleted from your account somewhere else. It is kept here, in this browser only.`);
      } else if (error instanceof ApiError && [409, 413, 422].includes(error.status)) {
        move(id, { type: "refused", reason: error.message });
      } else {
        move(id, { type: "failed" });
      }
    }
  }

  // ---- What the person asks for ---------------------------------------------------------------
  /** Puts a draft that is only here on the account. */
  function upload(id: string) {
    setNotice(null);
    move(id, { type: "upload" });
  }

  /** The person chose to keep their text over the one saved elsewhere. */
  function keepMine(id: string) {
    move(id, { type: "keep-mine" });
  }

  /** The person chose the text saved elsewhere. */
  async function takeTheirs(id: string) {
    const draft = find(id);
    if (!draft || draft.remote === undefined) return;
    try {
      const record = await withAccount((token) => gameApi.getDraft(draft.remote!, token));
      patch(id, (d) => ({
        ...d,
        name: record.name,
        rules: record.rules,
        skin: record.skin,
        shared: record.share_token,
        updated: Date.now(),
        sync: step(d.sync, { type: "take-theirs", version: record.version }),
      }));
    } catch {
      setNotice("We could not fetch the other version. Please try again in a moment.");
    }
  }

  /** Deletes the account's copy of a draft. True when there is none left, so the page can drop its own. */
  async function deleteRemote(draft: Draft): Promise<boolean> {
    if (draft.remote === undefined) return true;
    try {
      await withAccount((token) => gameApi.deleteDraft(draft.remote!, token));
      return true;
    } catch (error) {
      if (error instanceof ApiError && error.status === 404) return true;
      setNotice("We could not delete the draft from your account. Please try again in a moment.");
      return false;
    }
  }

  /** Makes the draft's private link, or finds the one it has. */
  async function share(id: string) {
    const draft = find(id);
    if (!draft || draft.remote === undefined) return;
    setNotice(null);
    try {
      const { token: secret } = await withAccount((token) => gameApi.shareDraft(draft.remote!, token));
      patch(id, (d) => ({ ...d, shared: secret }));
    } catch {
      setNotice("We could not make the link. Please try again in a moment.");
    }
  }

  /** Turns the private link off; it stops working at once. */
  async function unshare(id: string): Promise<boolean> {
    const draft = find(id);
    if (!draft || draft.remote === undefined) return false;
    setNotice(null);
    try {
      await withAccount((token) => gameApi.unshareDraft(draft.remote!, token));
      patch(id, (d) => ({ ...d, shared: null }));
      return true;
    } catch {
      setNotice("We could not turn the link off. Please try again in a moment.");
      return false;
    }
  }

  return { loaded, notice, setNotice, upload, keepMine, takeTheirs, deleteRemote, share, unshare };
}
