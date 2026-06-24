import { expect, test } from "bun:test";

import { createProject, openTerminal } from "./helpers";
import { getApp } from "./shared-app";

// A spawned terminal surface streams the daemon's shell through the orchestrator and paints xterm.
// Non-empty rendered text proves the end-to-end PTY path. Self-contained: creates its own project,
// then spawns a terminal into the default (empty) session.

test("a session terminal renders streamed output", async () => {
  const b = getApp();
  await createProject(b, `Terminal ${Date.now()}`);
  await openTerminal(b);

  // The spawned pane mounts the terminal; the daemon's shell streams through and paints xterm.
  const term = await b.$(".xterm");
  await term.waitForExist({ timeout: 20_000 });
  await b.waitUntil(async () => (await term.getText()).trim().length > 0, {
    timeout: 20_000,
    timeoutMsg: "terminal did not render streamed output",
  });

  expect((await term.getText()).trim().length).toBeGreaterThan(0);
}, 120_000);
