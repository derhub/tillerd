import { expect, test } from "bun:test";

import { createProject, launchReadyApp, openTerminal, uniqueName } from "./helpers";
import { getApp } from "./shared-app";

// end-to-end: the workspace-activity read-model stays live through the
// surface-status push (no user-initiated refetch), and view pointers survive a
// restart from the orchestrator settings store (not webview storage).

const dotSelector = '[data-testid="workspace-activity"]';

// xterm keyboard input is not drivable under tauri-webdriver; drive the PTY through
// the same IPC command the renderer uses.
async function sendInput(b: ReturnType<typeof getApp>, surface: string, text: string) {
  await b.execute(
    (key: string, bytes: number[]) => {
      const internals = (
        window as unknown as {
          __TAURI_INTERNALS__: { invoke: (cmd: string, args: unknown) => Promise<unknown> };
        }
      ).__TAURI_INTERNALS__;
      void internals.invoke("surface_channel_send", {
        key,
        msg: { kind: "input", bytes },
      });
    },
    surface,
    [...new TextEncoder().encode(text)],
  );
}

test("the activity dot follows surface status pushes, not user refetches", async () => {
  const b = getApp();
  await createProject(b, uniqueName("Activity"));

  // Spawning transitions pending -> live; the push invalidates the rollup and the
  // dot appears without any workspace mutation.
  const surface = await openTerminal(b);
  await b.waitUntil(
    async () => {
      const dot = await b.$(dotSelector);
      if (!(await dot.isExisting())) return false;
      return Number(await dot.getAttribute("data-running")) >= 1;
    },
    { timeout: 20_000, timeoutMsg: "activity dot did not appear after a surface went live" },
  );

  // A clean self-exit (typed into the shell, not a user command) must also push:
  // live -> idle, and the dot for this workspace clears. Enter is carriage return
  // under PTY line discipline, as xterm sends it.
  await sendInput(b, surface, "exit\r");
  await b.waitUntil(
    async () => {
      const dot = await b.$(dotSelector);
      if (!(await dot.isExisting())) return true;
      return Number(await dot.getAttribute("data-running")) === 0;
    },
    { timeout: 20_000, timeoutMsg: "activity dot did not clear after the PTY self-exited" },
  );
}, 120_000);

test("view pointers survive an app restart from the settings store", async () => {
  const project = uniqueName("Pointer");
  let sessionPath = "";

  const first = await launchReadyApp();
  try {
    const url = await createProject(first, project);
    sessionPath = `/session/${url.split("/session/")[1]}`;
    // Creating navigates into the session: the project is expanded and the
    // last-session pointer written; both persist via the settings store.
    await first.waitUntil(async () => (await first.$("body").getText()).includes(project), {
      timeout: 10_000,
      timeoutMsg: "created project did not appear in the sidebar",
    });
  } finally {
    await first.deleteSession();
  }

  const second = await launchReadyApp();
  try {
    // The exact session link is visible without any click: its project's
    // sidebar-expanded pointer was restored from the settings store.
    await second.waitUntil(
      async () => await (await second.$(`a[href$="${sessionPath}"]`)).isExisting(),
      {
        timeout: 15_000,
        timeoutMsg: "expanded-project pointer did not survive the restart",
      },
    );
    expect(await (await second.$(`a[href$="${sessionPath}"]`)).isExisting()).toBe(true);
  } finally {
    await second.deleteSession();
  }
}, 180_000);
