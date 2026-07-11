import { expect, test } from "bun:test";

import {
  createProject,
  launchReadyApp,
  observePause,
  openTerminal,
  surfaceId,
  uniqueName,
} from "./helpers";

// 0.0.17 Foundation integration: the "as one" proof. The per-flow specs each pin a single axis in
// isolation -- project-session (create), surface-isolation / session-revisit (switch + persistence),
// reload-deep-route (routing survives reload), panel-detach (multi-window parent reaction). This
// spec threads ONE project through all of them in a single continuous journey, so the assertion is
// not "each axis works" but "storage + state model + client engine compose": an entity created at
// the start keeps its identity across a session switch AND a full window reload.
//
// Own launch (like reload-deep-route): the reload would disturb the shared scenario app, so this
// drives its own app instance and tears it down in `finally`.
//
// Multi-window coherence is asserted through the only surface tauri-webdriver can observe -- the
// PARENT window's reaction to opening a project in a new window (WebDriver drives one webview and
// cannot see the child DOM or emit events; true cross-window invalidate->refetch is covered by the
// crossWindowSync unit tests and the desktop command-contract test).

test("storage, state model, and client engine compose across create/switch/reload/multi-window", async () => {
  const project = uniqueName("Foundation");
  const b = await launchReadyApp();
  try {
    // Create: project P opens its default session S1; spawn S1's terminal and capture its surface id.
    const url1 = await createProject(b, project);
    const s1 = await openTerminal(b);
    expect(s1).toBeTruthy();

    // A second session S2 in P with its own distinct surface (storage: two rows, two surfaces).
    const newSession = await b.$(`button[title="New session in ${project}"]`);
    await newSession.waitForExist({ timeout: 10_000 });
    await newSession.click();
    await b.waitUntil(
      async () => {
        const u = await b.getUrl();
        return u.includes("/session/") && u !== url1;
      },
      { timeout: 15_000, timeoutMsg: "second session in the project did not route anew" },
    );
    const s2 = await openTerminal(b);
    expect(s2).not.toBe(s1);

    // Switch back to S1: its own surface is restored from the store, not session 2's and not a fresh
    // one -- storage + cache carry identity across a client-side navigation.
    const id1 = url1.split("/session/")[1];
    const backToS1 = await b.$(`a[href$="/session/${id1}"]`);
    await backToS1.waitForExist({ timeout: 10_000 });
    await backToS1.click();
    await b.waitUntil(async () => (await b.getUrl()).endsWith(`/session/${id1}`), {
      timeout: 10_000,
      timeoutMsg: "did not navigate back to session 1",
    });
    await b.waitUntil(async () => (await surfaceId(b)) === s1, {
      timeout: 20_000,
      timeoutMsg: "session 1's surface was not restored after switching back",
    });

    // Reload at S1's deep route: the route survives (SPA fallback), the project is still in the
    // sidebar, and S1's surface is re-hydrated with the SAME id -- the full storage->state->cache
    // path survives a cold window reload, not just client routing.
    await b.refresh();
    await b.waitUntil(async () => (await b.getUrl()).endsWith(`/session/${id1}`), {
      timeout: 20_000,
      timeoutMsg: "deep session route was lost after reload",
    });
    await b.waitUntil(
      async () => {
        try {
          return (await b.$("body").getText()).includes(project);
        } catch {
          return false;
        }
      },
      { timeout: 20_000, timeoutMsg: "project did not re-render in the sidebar after reload" },
    );
    await b.waitUntil(async () => (await surfaceId(b)) === s1, {
      timeout: 20_000,
      timeoutMsg: "session 1's surface was not restored from storage after reload",
    });

    // Multi-window: opening P in a new window marks the parent project row (the observable
    // multi-window reaction). Re-attach afterwards so the child window does not linger.
    await b.execute((projectName: string) => {
      const heading = Array.from(document.querySelectorAll("span")).find(
        (el) => el.textContent === projectName,
      );
      heading?.dispatchEvent(
        new MouseEvent("contextmenu", { bubbles: true, clientX: 40, clientY: 40 }),
      );
    }, project);
    const openItem = await b.$('[role="menuitem"]*=Open in new window');
    await openItem.waitForExist({ timeout: 10_000 });
    await openItem.click();

    const indicator = await b.$('[data-testid="project-detached-indicator"]');
    await indicator.waitForExist({ timeout: 10_000 });
    expect(await indicator.isExisting()).toBe(true);

    await observePause(b);
    await indicator.click();
    await (
      await b.$('[data-testid="project-detached-indicator"]')
    ).waitForExist({ timeout: 10_000, reverse: true });
  } finally {
    await b.deleteSession();
  }
  // Higher ceiling than the single-phase own-launch specs: this journey chains a ~90s boot with
  // create + switch + reload + multi-window phases, so 180s could expire before a specific
  // `waitUntil` surfaces its own `timeoutMsg` -- an opaque bun:test timeout instead of a diagnostic.
}, 240_000);
