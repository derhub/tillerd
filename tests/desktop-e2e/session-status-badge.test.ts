import { expect, test } from "bun:test";

import { createProject, openTerminal, uniqueName } from "./helpers";
import { getApp } from "./shared-app";

// Session status badge (ui-panel-compound "Status badges" + the surface-status push channel): the
// sidebar session row carries a runtime dot whose `data-status` is driven purely by the
// orchestrator's surface-status channel (the sessionStatus store), never by a user refetch. A
// fresh session owns no surface, so its badge reads `idle`; spawning a terminal makes that
// session's surface go live and the badge must reach `running` on the push alone. The store's
// aggregation is unit-tested in isolation; only a wired push can actually move the rendered DOM,
// which is what this exercises end to end (channel -> store -> sidebar render).

test("spawning a terminal drives the session row badge to running via the surface-status push", async () => {
  const b = getApp();
  const url = await createProject(b, uniqueName("Badge"));
  const sessionId = url.split("/session/")[1]?.split(/[/?#]/)[0] ?? "";
  expect(sessionId).toBeTruthy();

  // The badge lives in the active project's session row (default-expanded), which mounts once the
  // project's sessions load -- wait for it rather than reading a mid-render snapshot.
  const badge = await b.$(`[data-tree-id="${sessionId}"] [data-testid="session-status"]`);
  await badge.waitForExist({ timeout: 15_000, timeoutMsg: "session row status badge never rendered" });

  // No surface bound yet -- the aggregate of an unknown session is `idle`, not a semantic hue.
  expect(await badge.getAttribute("data-status")).toBe("idle");

  await openTerminal(b);

  // The surface going live pushes {sessionId, surfaceId, status:"live"} down the channel; the
  // aggregated badge flips to `running` with no query. Poll past the transient `starting` state.
  await b.waitUntil(async () => (await badge.getAttribute("data-status")) === "running", {
    timeout: 20_000,
    timeoutMsg: "session badge did not reach running after the terminal surface went live",
  });
}, 120_000);
