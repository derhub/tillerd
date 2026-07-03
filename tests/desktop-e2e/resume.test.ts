import { expect, test } from "bun:test";

import { createProject, launchReadyApp } from "./helpers";

// Resume-after-restart: workspace state created in one app run survives a full restart. The app is
// launched twice against the SAME TILLERD_DIR (run.sh exports it), so the second launch is a genuine
// restart that must rehydrate the first launch's project from workspace persistence (tillerd.db).

test("a project survives an app restart", async () => {
  const project = `Resume ${Date.now()}`;

  // First launch: create the project, confirm it is in the sidebar, then close the app.
  const first = await launchReadyApp();
  try {
    await createProject(first, project);
    await first.waitUntil(async () => (await first.$("body").getText()).includes(project), {
      timeout: 10_000,
      timeoutMsg: "created project did not appear in the sidebar",
    });
  } finally {
    await first.deleteSession(); // closing the session closes the app -- the next launch is a restart
  }

  // Second launch (restart): the project must reappear without recreating it.
  const second = await launchReadyApp();
  try {
    await second.waitUntil(async () => (await second.$("body").getText()).includes(project), {
      timeout: 15_000,
      timeoutMsg: "project did not survive the restart — workspace persistence/resume regressed",
    });
    expect(await second.$("body").getText()).toContain(project);
  } finally {
    await second.deleteSession();
  }
}, 180_000);
