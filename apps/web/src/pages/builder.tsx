import {
  For,
  Show,
  createEffect,
  createMemo,
  createResource,
  createSignal,
  on,
  onCleanup,
} from "solid-js";
import { gameApi } from "../api/game";
import { ApiError } from "../api/http";
import type { FiledGame, Listing } from "../api/types";
import { withAccount } from "../auth/identity";
import { useAuth } from "../auth/session";
import { createAccountSync } from "../builder/account-sync";
import { checkSpec, type SpecCheck } from "../builder/engine";
import { createRunGuard } from "../builder/run-guard";
import { formatJson } from "../builder/format";
import { readSkin } from "../builder/skin";
import { LOCAL, type Sync, describe, onAccount, step } from "../builder/sync";
import { type Draft, type Drafts, fileName, loadDrafts, newDraft, saveDrafts, uniqueName } from "../builder/drafts";
import { Playground } from "../builder/playground";
import { MyGames } from "../builder/my-games";
import { PublishForm } from "../builder/publish-form";
import {
  CLOSED,
  type Publishing,
  type PublishingEvent,
  describeDone,
  formOpen,
  step as stepPublishing,
  whyNotYet,
} from "../builder/publishing";
import { SpecEditor, type SpecEditorHandle } from "../builder/spec-editor";
import { previewSkin } from "../components/rules-text";
import Footer from "../components/Footer";
import Navbar from "../components/Navbar";
import { usePageTitle } from "../components/page-title";
import march from "@examples/march.json?raw";

/** How long typing must pause before the engine is asked. */
const CHECK_DELAY_MS = 350;
/** How long it waits after the last change before writing the drafts to the browser. */
const SAVE_DELAY_MS = 400;
/** Set while a game is being started and cleared once its board has drawn: a game that hangs the
 *  page is not started again by reloading it, which would leave no way to edit it. */
const RUN_GUARD = "hubbable.workbench-run";

const STARTER = "March (the smallest game)";

interface Run {
  /** What was run, to tell when the text has moved on. */
  rulesText: string;
  skinText: string;
  rules: unknown;
  skin: Record<string, unknown> | null;
}

function freshDrafts(): Drafts {
  const first = newDraft("March", march);
  return { drafts: [first], current: first.id };
}

function startingDrafts(): Drafts {
  const saved = loadDrafts();
  return saved.drafts.length > 0 ? saved : freshDrafts();
}

const guard = createRunGuard(RUN_GUARD);
const guardHolds = guard.holds;
const setGuard = guard.set;

/** The workbench: write a game as a spec, hear from the engine what is wrong with it, and play it. */
export function BuilderPage() {
  usePageTitle(() => "Workbench");
  const [store, setStore] = createSignal(startingDrafts());
  const [saved, setSaved] = createSignal(true);
  const current = createMemo<Draft | undefined>(() => store().drafts.find((d) => d.id === store().current));
  // The page below is rebuilt when another draft comes on show, not on every keystroke.
  const currentId = createMemo(() => current()?.id);
  const draft = () => current() as Draft;
  const auth = useAuth();
  const signedIn = () => auth.account() !== null;
  /** What a draft made now starts as: for the account when someone is signed in. */
  const newSync = (): Sync => (signedIn() ? { kind: "dirty", version: null } : LOCAL);

  const [tab, setTab] = createSignal<"rules" | "look">("rules");
  const [check, setCheck] = createSignal<{ text: string; result: SpecCheck } | null>(null);
  const [running, setRunning] = createSignal<Run | null>(null);
  const [held, setHeld] = createSignal(guardHolds(store().current));
  const [starting, setStarting] = createSignal(false);
  const [confirmDelete, setConfirmDelete] = createSignal(false);
  const [notice, setNotice] = createSignal<string | null>(null);
  let rulesEditor: SpecEditorHandle | undefined;

  // ---- Keeping the drafts -----------------------------------------------------------------
  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  function persist() {
    clearTimeout(saveTimer);
    saveTimer = setTimeout(() => setSaved(saveDrafts(store())), SAVE_DELAY_MS);
  }
  onCleanup(() => {
    clearTimeout(saveTimer);
    saveDrafts(store());
  });
  const account = createAccountSync(
    {
      store,
      setStore,
      persist,
      signedIn,
      settled: () => auth.ready() && auth.working() === null,
      fresh: freshDrafts,
    },
    march,
  );
  function edit(patch: Partial<Pick<Draft, "name" | "rules" | "skin">>) {
    const id = store().current;
    setStore((now) => ({
      ...now,
      drafts: now.drafts.map((d) =>
        d.id === id ? { ...d, ...patch, updated: Date.now(), sync: step(d.sync, { type: "edit" }) } : d,
      ),
    }));
    persist();
  }
  function show(id: string) {
    setRunning(null);
    setCheck(null);
    setConfirmDelete(false);
    setNotice(null);
    publishStep({ type: "close" });
    setHeld(guardHolds(id));
    setStore((now) => ({ ...now, current: id }));
    persist();
  }
  function add(draft: Draft) {
    setStore((now) => ({ drafts: [...now.drafts, draft], current: draft.id }));
    setRunning(null);
    setCheck(null);
    setHeld(false);
    setStarting(false);
    setTab("rules");
    persist();
  }
  /** Takes a draft out of the workbench. The account's copy is the caller's business. */
  function drop(id: string) {
    const rest = store().drafts.filter((d) => d.id !== id);
    setRunning(null);
    setCheck(null);
    if (rest.length === 0) setStore(freshDrafts());
    else setStore({ drafts: rest, current: rest[0].id });
    setHeld(guardHolds(store().current));
    persist();
  }
  async function remove() {
    const gone = current();
    if (!gone) return;
    if (!(await account.deleteRemote(gone))) return;
    setConfirmDelete(false);
    drop(gone.id);
  }

  // ---- Publishing -------------------------------------------------------------------------
  const [publishing, setPublishing] = createSignal<Publishing>(CLOSED);
  const publishStep = (event: PublishingEvent) => setPublishing((now) => stepPublishing(now, event));
  const [busy, setBusy] = createSignal<string | null>(null);
  // The author's games that have left the drafts, asked for once the account's drafts are in.
  const [myGames, { refetch: refetchGames }] = createResource(
    () => signedIn() && account.loaded(),
    (ready): Promise<FiledGame[]> | FiledGame[] =>
      ready ? withAccount((token) => gameApi.listMyGames(token)).catch((): FiledGame[] => []) : [],
  );
  const games = () => myGames() ?? [];
  /** An author with a game that has been looked at is not held up again (the server decides; this is
   *  only what the form says to expect). */
  const trusted = () => games().some((game) => game.revisions > 0);
  const revisionOf = () => {
    const id = current()?.revises;
    return id ? games().find((game) => game.id === id) : undefined;
  };
  const blocked = () => {
    const now = current();
    return now ? whyNotYet(now, verdict()?.ok === true, skinRead().problem === null) : null;
  };

  async function send(listing: Listing) {
    const now = current();
    if (!now || now.remote === undefined || now.sync.kind !== "synced") return;
    const [remote, version, name] = [now.remote, now.sync.version, now.name];
    publishStep({ type: "send" });
    try {
      const filed = await withAccount((token) => gameApi.publishDraft(remote, version, listing, token));
      publishStep({ type: "filed", filed, name });
      // It is no longer a draft on the account, so it is no longer one here.
      drop(now.id);
      void refetchGames();
    } catch (error) {
      publishStep({
        type: "refused",
        reason: error instanceof ApiError ? error.message : "We could not reach the server. Please try again in a moment.",
      });
    }
  }

  /** Brings a game's working copy on show, after the account's drafts have been read again. */
  async function showWorkingCopy(remote: string) {
    await account.reload();
    const found = store().drafts.find((d) => d.remote === remote);
    if (found) show(found.id);
    void refetchGames();
  }
  async function revise(game: FiledGame) {
    setBusy(game.id);
    try {
      const copy = game.working_copy ?? (await withAccount((token) => gameApi.reviseGame(game.id, token))).id;
      await showWorkingCopy(copy);
    } catch (error) {
      account.setNotice(error instanceof ApiError ? error.message : "We could not start a new revision. Please try again in a moment.");
    } finally {
      setBusy(null);
    }
  }
  async function withdraw(game: FiledGame) {
    setBusy(game.id);
    try {
      await withAccount((token) => gameApi.withdrawDraft(game.id, token));
      await showWorkingCopy(game.id);
    } catch (error) {
      account.setNotice(error instanceof ApiError ? error.message : "We could not take it back. Please try again in a moment.");
    } finally {
      setBusy(null);
    }
  }

  // ---- Asking the engine ------------------------------------------------------------------
  let ticket = 0;
  createEffect(
    on(
      () => current()?.rules,
      (text) => {
        if (text === undefined) return;
        const mine = ++ticket;
        const timer = setTimeout(() => {
          checkSpec(text)
            .then((result) => mine === ticket && setCheck({ text, result }))
            .catch(
              () =>
                mine === ticket &&
                setCheck({
                  text,
                  result: {
                    ok: false,
                    message: "The engine could not be loaded, so this cannot be checked. Reload the page to try again.",
                    line: null,
                    column: null,
                    path: null,
                  },
                }),
            );
        }, CHECK_DELAY_MS);
        onCleanup(() => clearTimeout(timer));
      },
    ),
  );

  /** What the engine said about the text on show, once it has said it. */
  const verdict = () => {
    const now = check();
    return now && now.text === current()?.rules ? now.result : null;
  };
  const skinRead = createMemo(() => readSkin(current()?.skin ?? ""));
  const canRun = () => verdict()?.ok === true && skinRead().problem === null;
  const stale = () => {
    const run = running();
    const draft = current();
    return run !== null && draft !== undefined && (run.rulesText !== draft.rules || run.skinText !== draft.skin);
  };

  function run() {
    const draft = current();
    if (!draft || !canRun()) return;
    setHeld(false);
    setGuard(draft.id);
    setRunning({
      rulesText: draft.rules,
      skinText: draft.skin,
      rules: JSON.parse(draft.rules),
      skin: skinRead().skin,
    });
  }
  // The first time a game is good, it starts. After that, changes wait for the button, so
  // typing never throws a game in progress away.
  createEffect(() => {
    if (running() === null && canRun() && !held()) run();
  });

  // ---- Sharing ----------------------------------------------------------------------------
  const [copied, setCopied] = createSignal(false);
  const shareUrl = (secret: string) => `${window.location.origin}/build/shared/${secret}`;
  async function copy(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      account.setNotice("The link could not be copied. Select it and copy it by hand.");
    }
  }
  /** A new link, and the old one stops working. */
  async function renew(id: string) {
    if (await account.unshare(id)) await account.share(id);
  }

  // ---- Starting points --------------------------------------------------------------------
  const [catalogue] = createResource(() => gameApi.getCatalog().catch(() => []));
  const [startFrom, setStartFrom] = createSignal(STARTER);
  const names = () => store().drafts.map((d) => d.name);

  async function create() {
    setNotice(null);
    const from = startFrom();
    if (from === STARTER) return add(newDraft(uniqueName("March", names()), march, "", newSync()));
    try {
      const detail = await gameApi.getGame(from);
      add(
        newDraft(
          uniqueName(`${detail.page.name} (copy)`, names()),
          formatJson(detail.spec.rules),
          previewSkin(detail) ? formatJson(previewSkin(detail)) : "",
          newSync(),
        ),
      );
    } catch {
      setNotice("We could not fetch that game. Please try again in a moment.");
    }
  }

  async function open(file: File | undefined) {
    if (!file) return;
    add(newDraft(uniqueName(file.name.replace(/\.json$/i, ""), names()), await file.text(), "", newSync()));
  }

  function download(name: string, text: string) {
    const url = URL.createObjectURL(new Blob([text], { type: "application/json" }));
    const link = document.createElement("a");
    link.href = url;
    link.download = name;
    link.click();
    URL.revokeObjectURL(url);
  }

  function format() {
    const draft = current();
    if (!draft) return;
    const target = tab() === "rules" ? draft.rules : draft.skin;
    try {
      const pretty = formatJson(JSON.parse(target));
      edit(tab() === "rules" ? { rules: pretty } : { skin: pretty });
    } catch {
      setNotice("Only text that is already JSON can be tidied. Fix the problem below first.");
    }
  }

  const problemLine = () => {
    const now = verdict();
    return now && !now.ok ? now.line : null;
  };

  return (
    <div class="page">
      <Navbar activeLink="build" />
      <main class="builder">
        <header class="builder-head">
          <h1 class="builder-title">Workbench</h1>
          <p>
            Write a game as a spec, hear from the engine what is wrong with it, and play it here against yourself.{" "}
            <Show when={signedIn()} fallback="Drafts stay in this browser unless you sign in.">
              Drafts are saved to your account.
            </Show>
          </p>
          <p class="builder-links">
            <a href="/docs/01-first-game.html" class="text-link">Your first game</a>
            <a href="/docs/reference.html" class="text-link">The spec reference</a>
          </p>
        </header>

        <Show when={(() => { const now = publishing(); return now.kind === "done" ? now : null; })()}>
          {(done) => (
            <p class="builder-done" role="status">
              <span>{describeDone(done())}</span>
              <button type="button" class="builder-small" onClick={() => publishStep({ type: "dismiss" })}>
                Dismiss
              </button>
            </p>
          )}
        </Show>
        <Show when={games().length > 0}>
          <MyGames games={games()} busy={busy()} onEdit={(game) => void revise(game)} onWithdraw={(game) => void withdraw(game)} />
        </Show>

        <Show when={currentId()} keyed>
          {(id) => (
            <div class="builder-grid" data-draft={id}>
              <section class="builder-panel" aria-label="The game's spec">
                <div class="builder-bar">
                  <label class="builder-field">
                    <span class="visually-hidden">Draft</span>
                    <select value={draft().id} onChange={(event) => show(event.currentTarget.value)} aria-label="Draft">
                      <For each={store().drafts}>{(d) => <option value={d.id} selected={d.id === draft().id}>{d.name}</option>}</For>
                    </select>
                  </label>
                  <input
                    class="builder-name"
                    aria-label="Draft name"
                    value={draft().name}
                    onInput={(event) => edit({ name: event.currentTarget.value })}
                  />
                  <button type="button" class="builder-small" onClick={() => setStarting(!starting())} aria-expanded={starting()}>
                    New draft
                  </button>
                  <label class="builder-small builder-file">
                    Open a file…
                    <input
                      type="file"
                      accept=".json,application/json"
                      class="visually-hidden"
                      onChange={(event) => {
                        void open(event.currentTarget.files?.[0]);
                        event.currentTarget.value = "";
                      }}
                    />
                  </label>
                </div>

                <div class="builder-account">
                  <Show
                    when={onAccount(draft().sync)}
                    fallback={
                      <Show
                        when={signedIn()}
                        fallback={
                          <span>
                            Only in this browser. <a class="text-link" href="/sign-in?next=%2Fbuild">Sign in</a> to keep it on your account and to share a link to it.
                          </span>
                        }
                      >
                        <span>Only in this browser.</span>
                        <button type="button" class="builder-small" onClick={() => account.upload(draft().id)}>
                          Save to my account
                        </button>
                      </Show>
                    }
                  >
                    <span>{describe(draft().sync)}</span>
                    <Show when={!formOpen(publishing())}>
                      <button type="button" class="builder-small" disabled={blocked() !== null} onClick={() => publishStep({ type: "open" })}>
                        Publish…
                      </button>
                      <Show when={blocked()}>{(why) => <span class="builder-publish-why">{why()}</span>}</Show>
                    </Show>
                  </Show>
                </div>
                <Show when={draft().revises}>
                  <p class="builder-share-note">
                    A new revision of <strong>{revisionOf()?.name ?? "a game on the hub"}</strong>. Nothing changes on the hub until you publish it.
                  </p>
                </Show>
                <Show when={draft().rejection}>
                  {(reason) => (
                    <p class="builder-warning" role="status">
                      <strong>Sent back.</strong> {reason()}
                    </p>
                  )}
                </Show>
                <Show when={formOpen(publishing())}>
                  <PublishForm
                    name={draft().name}
                    initial={draft().listing}
                    revision={draft().revises !== null}
                    review={!trusted()}
                    state={publishing()}
                    onSend={(listing) => void send(listing)}
                    onCancel={() => publishStep({ type: "close" })}
                  />
                </Show>
                <Show when={draft().sync.kind === "conflict"}>
                  <div class="builder-warning builder-conflict" role="alert">
                    <p>This draft was changed somewhere else too. Which version do you want?</p>
                    <div class="builder-conflict-actions">
                      <button type="button" class="builder-small" onClick={() => account.keepMine(draft().id)}>
                        Keep the one here
                      </button>
                      <button type="button" class="builder-small" onClick={() => void account.takeTheirs(draft().id)}>
                        Load the other one
                      </button>
                    </div>
                  </div>
                </Show>
                <Show when={account.notice()}>{(text) => <p class="builder-warning" role="alert">{text()}</p>}</Show>

                <Show when={starting()}>
                  <div class="builder-start">
                    <label>
                      Start from{" "}
                      <select value={startFrom()} onChange={(event) => setStartFrom(event.currentTarget.value)}>
                        <option value={STARTER}>{STARTER}</option>
                        <For each={catalogue() ?? []}>{(game) => <option value={game.slug}>A copy of {game.name}</option>}</For>
                      </select>
                    </label>
                    <button type="button" class="btn-signup" onClick={() => void create()}>
                      Create draft
                    </button>
                  </div>
                </Show>

                <div class="builder-tabs" role="tablist" aria-label="What to edit">
                  <button
                    type="button"
                    role="tab"
                    aria-selected={tab() === "rules"}
                    class="builder-tab"
                    onClick={() => setTab("rules")}
                  >
                    Rules
                    <Show when={verdict() && !verdict()!.ok}>
                      <span class="tab-flag"><span class="visually-hidden"> has a problem</span></span>
                    </Show>
                  </button>
                  <button
                    type="button"
                    role="tab"
                    aria-selected={tab() === "look"}
                    class="builder-tab"
                    onClick={() => setTab("look")}
                  >
                    Look
                    <Show when={skinRead().problem}>
                      <span class="tab-flag"><span class="visually-hidden"> has a problem</span></span>
                    </Show>
                  </button>
                  <button type="button" class="builder-small builder-tidy" onClick={format}>
                    Tidy up
                  </button>
                </div>

                <div class="builder-status" role="status" classList={{ "is-bad": tab() === "rules" ? verdict()?.ok === false : skinRead().problem !== null }}>
                  <Show
                    when={tab() === "rules"}
                    fallback={
                      <Show when={skinRead().problem} fallback={<span>{draft().skin.trim() === "" ? "No look set: the default one is used." : "The look is readable."}</span>}>
                        {(problem) => <span><strong>This look cannot be read.</strong> {problem()}</span>}
                      </Show>
                    }
                  >
                    <Show when={verdict()} fallback={<span>Checking…</span>}>
                      {(result) => (
                        <Show
                          when={!result().ok ? (result() as Extract<SpecCheck, { ok: false }>) : null}
                          fallback={
                            <span>
                              <strong>This is a game.</strong>{" "}
                              {(() => {
                                const good = result() as Extract<SpecCheck, { ok: true }>;
                                return `${good.name}, for ${good.players.join(" and ")}.`;
                              })()}
                            </span>
                          }
                        >
                          {(problem) => (
                            <>
                              <span>
                                <strong>The engine cannot read this.</strong> {problem().message}
                                <Show when={problem().path}>
                                  {" "}In <code>{problem().path}</code>.
                                </Show>
                              </span>
                              <Show when={problem().line}>
                                {(line) => (
                                  <button type="button" class="builder-small" onClick={() => rulesEditor?.goTo(line())}>
                                    Go to line {line()}
                                  </button>
                                )}
                              </Show>
                            </>
                          )}
                        </Show>
                      )}
                    </Show>
                  </Show>
                </div>

                <Show
                  when={tab() === "rules"}
                  fallback={
                    <SpecEditor
                      label="The board's look, as JSON"
                      value={draft().skin}
                      placeholder={'Optional. How the board and its pieces are drawn, for example { "pieces": { "MARCHER": "♟" } }. Empty uses the default look.'}
                      onInput={(skin) => edit({ skin })}
                      onRun={run}
                    />
                  }
                >
                  <SpecEditor
                    ref={(handle) => (rulesEditor = handle)}
                    label="The game's rules, as JSON"
                    value={draft().rules}
                    flag={problemLine()}
                    onInput={(rules) => edit({ rules })}
                    onRun={run}
                  />
                </Show>

                <div class="builder-foot">
                  <button type="button" class="builder-small" onClick={() => download(fileName(draft().name), draft().rules)}>
                    Download the rules
                  </button>
                  <Show when={draft().skin.trim() !== ""}>
                    <button type="button" class="builder-small" onClick={() => download(fileName(`${draft().name} look`), draft().skin)}>
                      Download the look
                    </button>
                  </Show>
                  <Show when={draft().remote !== undefined}>
                    <Show
                      when={draft().shared}
                      fallback={
                        <button type="button" class="builder-small" onClick={() => void account.share(draft().id)}>
                          Share a link
                        </button>
                      }
                    >
                      {(secret) => (
                        <span class="builder-share">
                          <input
                            class="builder-link"
                            readOnly
                            aria-label="Private link"
                            value={shareUrl(secret())}
                            onFocus={(event) => event.currentTarget.select()}
                          />
                          <button type="button" class="builder-small" onClick={() => void copy(shareUrl(secret()))}>
                            {copied() ? "Copied" : "Copy link"}
                          </button>
                          <button type="button" class="builder-small" onClick={() => void account.unshare(draft().id)}>
                            Stop sharing
                          </button>
                          <button type="button" class="builder-small" onClick={() => void renew(draft().id)}>
                            Make a new link
                          </button>
                        </span>
                      )}
                    </Show>
                  </Show>
                  <Show
                    when={confirmDelete()}
                    fallback={
                      <button type="button" class="builder-small builder-delete" onClick={() => setConfirmDelete(true)}>
                        Delete this draft
                      </button>
                    }
                  >
                    <span class="builder-confirm">
                      Delete “{draft().name}” for good?
                      <button type="button" class="builder-small builder-delete" onClick={remove}>
                        Yes, delete it
                      </button>
                      <button type="button" class="builder-small" onClick={() => setConfirmDelete(false)}>
                        Keep it
                      </button>
                    </span>
                  </Show>
                </div>
                <Show when={draft().shared}>
                  <p class="builder-share-note">
                    Anyone with this link can play the game, with no account. They see the version last saved to your account.
                  </p>
                </Show>
                <Show when={!saved()}>
                  <p class="builder-warning" role="alert">
                    This browser would not keep your drafts, so they will be gone when you leave. Download your work.
                  </p>
                </Show>
              </section>

              <section class="builder-play" aria-label="Play the game">
                <div class="builder-run">
                  <button
                    type="button"
                    class="btn-primary-lg"
                    disabled={!canRun() || (running() !== null && !stale())}
                    onClick={run}
                  >
                    {running() === null ? "Run" : stale() ? "Run these changes" : "Running"}
                  </button>
                  <span class="builder-hint">
                    <Show when={stale()} fallback={"Reset, under the board, plays it again. Ctrl or ⌘ + Enter runs changes."}>
                      The board is still playing the version you last ran.
                    </Show>
                  </span>
                </div>
                <Show when={notice()}>{(text) => <p class="builder-warning" role="alert">{text()}</p>}</Show>
                <Show when={held()}>
                  <p class="builder-warning" role="alert">
                    This game did not finish starting last time, so it was not started now. Change it, then run it.
                  </p>
                </Show>
                <Show
                  when={running()}
                  keyed
                  fallback={
                    <p class="feed-message">
                      {verdict()?.ok === false
                        ? "Fix what the engine reports, and your game will start here."
                        : "Your game plays here."}
                    </p>
                  }
                >
                  {(now) => <Playground rules={now.rules} skin={now.skin} onAlive={() => setGuard(null)} />}
                </Show>
              </section>
            </div>
          )}
        </Show>
      </main>
      <Footer />
    </div>
  );
}
