import { expect, test } from "bun:test";

import { createProject, launchReadyApp, uniqueName, type Browser } from "./helpers";

// Own-launch spec (run without the shared-app preload): view pointers survive a
// restart from the orchestrator settings store, not webview storage.

// Pointer writes are fire-and-forget; poll the persisted settings until this
// session's last-session pointer and its project's expanded flag are on disk
// before closing the app. The invoke result cannot be awaited inline under
// tauri-webdriver; each poll fires it into a window slot and reads the slot's
// prior value (the command-center spec's pattern).
async function waitForPersistedPointers(b: Browser, sessionId: string): Promise<void> {
  await b.waitUntil(
    async () =>
      await b.execute((sid: string) => {
        const w = window as unknown as {
          __TAURI_INTERNALS__: { invoke(cmd: string, args: unknown): Promise<unknown> };
          __pointersPersisted?: boolean;
        };
        void w.__TAURI_INTERNALS__
          .invoke("setting_list", { scope: "global", projectId: null })
          .then((entries) => {
            const list = entries as { key: string; value: unknown }[];
            const last = list.find(
              (e) => e.key.startsWith("view.last-session.") && e.value === sid,
            );
            const projectId = last?.key.slice("view.last-session.".length);
            const expanded =
              projectId != null &&
              list.some((e) => e.key === `sidebar.expanded.${projectId}` && e.value === true);
            w.__pointersPersisted = expanded;
          })
          .catch(() => {
            w.__pointersPersisted = false;
          });
        return w.__pointersPersisted ?? false;
      }, sessionId),
    { timeout: 15_000, timeoutMsg: "view pointers did not persist before shutdown" },
  );
}

test("view pointers survive an app restart from the settings store", async () => {
  const project = uniqueName("Pointer");
  let sessionPath = "";

  const first = await launchReadyApp();
  try {
    const url = await createProject(first, project);
    const sessionId = url.split("/session/")[1];
    sessionPath = `/session/${sessionId}`;
    // Creating navigates into the session: the project is expanded and the
    // last-session pointer written; both persist via the settings store.
    await first.waitUntil(async () => (await first.$("body").getText()).includes(project), {
      timeout: 10_000,
      timeoutMsg: "created project did not appear in the sidebar",
    });
    await waitForPersistedPointers(first, sessionId);
  } finally {
    await first.deleteSession();
  }

  const second = await launchReadyApp();
  try {
    // The exact session link is visible without any click: its project's
    // sidebar-expanded pointer was restored from the settings store.
    try {
      await second.waitUntil(
        async () => await (await second.$(`a[href$="${sessionPath}"]`)).isExisting(),
        { timeout: 15_000 },
      );
    } catch {
      const hasProject = (await second.$("body").getText()).includes(project);
      const toggles = await second.execute(() =>
        [...document.querySelectorAll('[data-testid="project-expand"]')].map((el) => ({
          id: el.getAttribute("data-project-id"),
          label: el.getAttribute("aria-label") ?? el.textContent,
        })),
      );
      throw new Error(
        `expanded-project pointer did not survive the restart ` +
          `(project row visible: ${hasProject}; toggles: ${JSON.stringify(toggles)})`,
      );
    }
    expect(await (await second.$(`a[href$="${sessionPath}"]`)).isExisting()).toBe(true);
  } finally {
    await second.deleteSession();
  }
}, 180_000);
