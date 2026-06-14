import { afterEach, expect, test } from "bun:test";
import { type Browser, createProject, launchReadyApp, openTerminal } from "./helpers";

// The notification bell sits in the shell's bottom-right cluster. A live lifecycle event (here, a
// spawned terminal -> "surface-started") raises an unread badge; clicking the bell opens the
// popover, clears the unread count, and a session notification's row client-side navigates to that
// session. Popover open, the unread badge, and in-portal link navigation are real-webview concerns
// (happy-dom has no layout/router), so they live here; row content is unit-tested.

let browser: Browser | undefined;
afterEach(async () => {
  await browser?.deleteSession();
  browser = undefined;
});

test("a live event badges the bell; opening clears it and a session row navigates", async () => {
  const b = await launchReadyApp();
  browser = b;

  // Create a session and spawn a terminal — the spawn raises a live "surface-started"
  // notification carrying this session id.
  const sessionUrl = await createProject(b, "Notif E2E");
  const sessionId = sessionUrl.split("/session/")[1]?.split(/[/?#]/)[0] ?? "";
  expect(sessionId).not.toBe("");
  await openTerminal(b);

  // The live event increments the unread badge.
  const badge = await b.$('[data-testid="notification-unread"]');
  await badge.waitForExist({ timeout: 15_000 });

  // Leave the session so the click-through has somewhere to navigate from.
  await b.execute(() => {
    window.history.pushState({}, "", "/");
    window.dispatchEvent(new Event("popstate"));
  });
  await b.waitUntil(async () => !(await b.getUrl()).includes("/session/"), {
    timeout: 10_000,
    timeoutMsg: "did not leave the session route",
  });

  // Open the bell: the popover lists notifications and opening clears the unread count.
  await (await b.$('[aria-label^="Notifications"]')).click();
  const panel = await b.$('[data-slot="popover-content"]');
  await panel.waitForExist({ timeout: 10_000 });
  await b.waitUntil(
    async () => !(await (await b.$('[data-testid="notification-unread"]')).isExisting()),
    { timeout: 10_000, timeoutMsg: "unread badge did not clear on open" },
  );

  // The session notification row client-side navigates back to that session (no hard tauri:// load).
  const row = await b.$(`a[href="/session/${sessionId}"]`);
  await row.waitForExist({ timeout: 10_000 });
  await row.click();
  await b.waitUntil(async () => (await b.getUrl()).includes(`/session/${sessionId}`), {
    timeout: 10_000,
    timeoutMsg: "notification row did not navigate to its session",
  });
  // The app is still alive (client nav, not a hard load to a dead custom-scheme URL).
  expect(await (await b.$('[aria-label^="Notifications"]')).isExisting()).toBe(true);
}, 120_000);
