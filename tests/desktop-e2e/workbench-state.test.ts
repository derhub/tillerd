import { expect, test } from "bun:test";

import { launchReadyApp, openView, type Browser } from "./helpers";

// Own-launch spec (like resume.test.ts / view-pointers-restart.test.ts): the workbench chrome
// layout (active sidebar view, bottom-panel visibility) is a global settings-store value,
// restored on every launch the same way theme and view pointers are restored -- this proves that
// durability for the workbench keys specifically, across a genuine process restart. There is no
// URL for this state (it is app chrome, not routing), so a client-nav reload cannot exercise it;
// only a real restart can.

// Settings writes are fire-and-forget (see the settings-context comment); poll the persisted
// store until this run's writes have landed before closing the app -- otherwise the shutdown can
// race ahead of the write and this spec would flake on nothing but timing (same pattern as
// view-pointers-restart's waitForPersistedPointers).
async function waitForPersistedWorkbenchState(b: Browser): Promise<void> {
  await b.waitUntil(
    async () =>
      await b.execute(() => {
        const w = window as unknown as {
          __TAURI_INTERNALS__: { invoke(cmd: string, args: unknown): Promise<unknown> };
          __workbenchPersisted?: boolean;
        };
        void w.__TAURI_INTERNALS__
          .invoke("setting_list", { scope: "global", projectId: null })
          .then((entries) => {
            const list = entries as { key: string; value: unknown }[];
            const view = list.find((e) => e.key === "workbench.view")?.value;
            const visible = list.find((e) => e.key === "workbench.panel.visible")?.value;
            w.__workbenchPersisted = view === "commands" && visible === true;
          })
          .catch(() => {
            w.__workbenchPersisted = false;
          });
        return w.__workbenchPersisted ?? false;
      }),
    { timeout: 15_000, timeoutMsg: "workbench layout did not persist before shutdown" },
  );
}

test("active view and bottom-panel visibility survive an app restart", async () => {
  const first = await launchReadyApp();
  try {
    await openView(first, "Commands");
    await (
      await first.$('[data-testid="commands-empty"], [data-testid="commands-list"]')
    ).waitForExist({ timeout: 10_000, timeoutMsg: "Commands view did not render" });

    await (await first.$('[aria-label="Toggle bottom panel"]')).click();
    await (await first.$('[data-testid="log-viewer"]')).waitForExist({
      timeout: 10_000,
      timeoutMsg: "bottom panel did not open on the Logs tab",
    });

    await waitForPersistedWorkbenchState(first);
  } finally {
    await first.deleteSession();
  }

  const second = await launchReadyApp();
  try {
    const commandsButton = await second.$(
      '[role="toolbar"][aria-label="Views"] button[aria-label="Commands"]',
    );
    await commandsButton.waitForExist({ timeout: 15_000 });
    expect(await commandsButton.getAttribute("aria-pressed")).toBe("true");
    await (
      await second.$('[data-testid="commands-empty"], [data-testid="commands-list"]')
    ).waitForExist({ timeout: 15_000, timeoutMsg: "Commands view was not restored after restart" });
    await (await second.$('[data-testid="log-viewer"]')).waitForExist({
      timeout: 15_000,
      timeoutMsg: "bottom panel (Logs) did not stay open after restart",
    });
  } finally {
    await second.deleteSession();
  }
}, 180_000);
