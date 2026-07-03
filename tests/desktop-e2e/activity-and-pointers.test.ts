import { test } from "bun:test";

import { createProject, openTerminal, uniqueName } from "./helpers";
import { getApp } from "./shared-app";

// The workspace-activity read-model stays live through the surface-status push: the
// dot appears when a spawn transitions pending -> live, and nothing else invalidates
// the ["workspaces", "activity"] key -- no workspace mutation runs here, so the dot
// appearing proves push -> invalidate -> refetch end to end. The other transition
// triggers (PTY self-exit, crash, stop) reuse the same pipeline and are covered by
// unit tests at each link (surface_channel exit tests, surfaceStatusSync coalescing,
// WorkspaceSwitcher rollup rendering); driving PTY input under tauri-webdriver is
// not reliable enough to re-prove them here.

const dotSelector = '[data-testid="workspace-activity"]';

test("the activity dot appears from the surface-status push, not a user refetch", async () => {
  const b = getApp();
  await createProject(b, uniqueName("Activity"));

  await openTerminal(b);
  await b.waitUntil(
    async () => {
      const dot = await b.$(dotSelector);
      if (!(await dot.isExisting())) return false;
      return Number(await dot.getAttribute("data-running")) >= 1;
    },
    { timeout: 20_000, timeoutMsg: "activity dot did not appear after a surface went live" },
  );
}, 120_000);
