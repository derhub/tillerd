import { expect, test } from "bun:test";

import { createProject, launchReadyApp, openView, uniqueName, type Browser } from "./helpers";

// Own-launch spec (like workbench-state.test.ts / resume.test.ts): a project's sidebar
// expand/collapse is a durable global setting (`sidebar.expanded.<id>`), absent-means-expanded, so
// only an explicit collapse is ever written. This drives the collapse through the real sidebar UI
// (a click on the project's chevron), then proves that write survives a genuine process restart --
// the fresh process's settings host still serves the collapsed value. Unlike the always-visible
// workbench chrome (workbench-state), the sidebar tree is workspace-scoped and re-expands a
// project on navigation, so the durable settings layer -- not a post-restart DOM snapshot -- is
// the stable, deterministic witness of restoration.

// The project owning a session row, read off the row's `data-parent-id`.
async function projectIdForSession(b: Browser, sessionId: string): Promise<string> {
  return b.execute((id: string) => {
    const row = document.querySelector(`[data-tree-id="${id}"]`);
    return row?.getAttribute("data-parent-id") ?? "";
  }, sessionId);
}

// Read `sidebar.expanded.<projectId>` from the running process's global settings host. Settings
// writes are fire-and-forget, so callers poll this until the expected value lands (same guard as
// workbench-state's waitForPersistedWorkbenchState) -- both to confirm the write before shutdown
// and to confirm the value is served again after the restart.
async function collapseSettingIsFalse(b: Browser, projectId: string): Promise<boolean> {
  return b.execute((pid: string) => {
    const w = window as unknown as {
      __TAURI_INTERNALS__: { invoke(cmd: string, args: unknown): Promise<unknown> };
      __expandProbe?: boolean;
    };
    void w.__TAURI_INTERNALS__
      .invoke("setting_list", { scope: "global", projectId: null })
      .then((entries) => {
        const list = entries as { key: string; value: unknown }[];
        w.__expandProbe = list.find((e) => e.key === `sidebar.expanded.${pid}`)?.value === false;
      })
      .catch(() => {
        w.__expandProbe = false;
      });
    return w.__expandProbe ?? false;
  }, projectId);
}

test("a collapsed project stays collapsed after an app restart", async () => {
  let projectId = "";
  const first = await launchReadyApp();
  try {
    // This own-launch spec runs after workbench-state persists `workbench.view=commands` globally,
    // so a fresh launch restores the Commands sidebar view -- where "New project" does not exist.
    // Force the Sessions view first (route "/" alone does not reset the active sidebar view).
    await openView(first, "Sessions");
    const url = await createProject(first, uniqueName("Collapse"));
    const sessionId = url.split("/session/")[1]?.split(/[/?#]/)[0] ?? "";
    projectId = await projectIdForSession(first, sessionId);
    expect(projectId).toBeTruthy();

    const toggle = await first.$(`[data-testid="project-expand"][data-project-id="${projectId}"]`);
    await toggle.waitForExist({ timeout: 10_000 });
    // Default is expanded (absent pointer), so the child session row is present first...
    expect(await toggle.getAttribute("aria-expanded")).toBe("true");
    await toggle.click();
    // ...and one click collapses it, unmounting the children.
    await first.waitUntil(async () => (await toggle.getAttribute("aria-expanded")) === "false", {
      timeout: 10_000,
      timeoutMsg: "project did not collapse on click",
    });
    await (
      await first.$(`[data-tree-id="${sessionId}"]`)
    ).waitForExist({
      timeout: 10_000,
      reverse: true,
      timeoutMsg: "collapsing did not unmount the child session row",
    });

    // The click's collapse write must land in the durable store before we kill the process.
    await first.waitUntil(async () => collapseSettingIsFalse(first, projectId), {
      timeout: 15_000,
      timeoutMsg: "collapse state did not persist before shutdown",
    });
  } finally {
    await first.deleteSession();
  }

  const second = await launchReadyApp();
  try {
    // A genuine restart: the fresh process re-opens the settings host and must still serve the
    // collapsed value written by the previous process.
    await second.waitUntil(async () => collapseSettingIsFalse(second, projectId), {
      timeout: 15_000,
      timeoutMsg: "collapse state was not restored after the app restart",
    });
  } finally {
    await second.deleteSession();
  }
}, 180_000);
