import { expect, test } from "bun:test";

import { createProject, launchReadyApp } from "./helpers";

// SPA fallback: file-based routing uses real path routes (/session/$id). A window
// navigated client-side to a deep route and then reloaded must still serve the app -- this relies
// on Tauri v2's built-in index.html asset fallback. Its own launch (like resume) so the reload does
// not disturb the shared scenario app.

test("a deep session route survives a window reload", async () => {
  const project = `Reload ${Date.now()}`;
  const b = await launchReadyApp();
  try {
    // createProject navigates to the new session's deep route.
    const url = await createProject(b, project);
    const sessionPath = `/session/${url.split("/session/")[1]}`;
    expect(await b.getUrl()).toContain(sessionPath);

    // Reload at the deep route. Without the fallback this 404s to a blank window.
    await b.refresh();

    await b.waitUntil(async () => (await b.getUrl()).includes(sessionPath), {
      timeout: 20_000,
      timeoutMsg: "deep route was lost after reload — Tauri index.html fallback missing?",
    });
    await b.waitUntil(async () => (await b.$("body").getText()).includes(project), {
      timeout: 20_000,
      timeoutMsg: "shell did not re-render after reloading at the deep route",
    });
    expect(await b.$("body").getText()).toContain(project);
  } finally {
    await b.deleteSession();
  }
}, 180_000);
