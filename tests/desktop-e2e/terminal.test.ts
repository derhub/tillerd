import { afterEach, expect, test } from "bun:test";
import { type Browser, createProject, launchReadyApp } from "./helpers";

// A session pane creates a terminal surface; the daemon's shell streams through the orchestrator and
// paints xterm. Non-empty rendered text proves the end-to-end PTY path. Self-contained: creates its
// own project (whose default session opens a terminal) rather than relying on prior tests' state.

let browser: Browser | undefined;
afterEach(async () => {
  await browser?.deleteSession();
  browser = undefined;
});

test("a session terminal renders streamed output", async () => {
  const b = (browser = await launchReadyApp());
  await createProject(b, `Terminal ${Date.now()}`);

  // The session route mounts the terminal; the daemon's shell streams through and paints xterm.
  const term = await b.$(".xterm");
  await term.waitForExist({ timeout: 20_000 });
  await b.waitUntil(async () => (await term.getText()).trim().length > 0, {
    timeout: 20_000,
    timeoutMsg: "terminal did not render streamed output",
  });

  expect((await term.getText()).trim().length).toBeGreaterThan(0);
}, 120_000);
