import { afterEach, expect, test } from "bun:test";
import { type Browser, createProject, launchReadyApp, openTerminal } from "./helpers";

// A spawned terminal surface streams the daemon's shell through the orchestrator and paints xterm.
// Non-empty rendered text proves the end-to-end PTY path. Self-contained: creates its own project,
// then spawns a terminal into the default (empty) session.

let browser: Browser | undefined;
afterEach(async () => {
  await browser?.deleteSession();
  browser = undefined;
});

test("a session terminal renders streamed output", async () => {
  const b = (browser = await launchReadyApp());
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
