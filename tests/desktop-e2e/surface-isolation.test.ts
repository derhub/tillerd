import { expect, test } from "bun:test";

import { createProject, openTerminal } from "./helpers";
import { getApp } from "./shared-app";

// Surface isolation: each session drives its OWN terminal surface. A spawned pane exposes its
// surface id as `data-surface-id`; two sessions in the same project must mount two DISTINCT
// surfaces. A shared surface would reuse the first id. The live counterpart of the data-layer test
// `each_session_gets_its_own_distinct_surface`.

test("each session mounts its own distinct surface", async () => {
  const b = getApp();
  const project = `Isolation ${Date.now()}`;

  const session1Url = await createProject(b, project);
  const s1 = await openTerminal(b);

  const newSession = await b.$(`button[title="New session in ${project}"]`);
  await newSession.waitForExist({ timeout: 10_000 });
  await newSession.click();
  await b.waitUntil(
    async () => {
      const url = await b.getUrl();
      return url.includes("/session/") && url !== session1Url;
    },
    { timeout: 15_000, timeoutMsg: "second session did not route to its own surface" },
  );
  const s2 = await openTerminal(b);

  expect(s1).toBeTruthy();
  expect(s2).toBeTruthy();
  expect(s2).not.toBe(s1);
}, 120_000);
