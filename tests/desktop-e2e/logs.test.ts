import { afterEach, expect, test } from "bun:test";
import { type Browser, launchReadyApp } from "./helpers";

// The log viewer is reached via the native View > Logs menu, which is not WebDriver-
// accessible (native chrome lives outside the webview). So this drives the route directly:
// navigate to /logs and assert the viewer renders. The menu's renderer handler
// (menu:navigate -> navigate) is a manual/visual check.

let browser: Browser | undefined;
afterEach(async () => {
  await browser?.deleteSession();
  browser = undefined;
});

test("the /logs route renders the log viewer", async () => {
  const b = await launchReadyApp();
  browser = b;

  const origin = new URL(await b.getUrl()).origin;
  await b.url(`${origin}/logs`);

  await b.waitUntil(async () => (await b.getUrl()).includes("/logs"), {
    timeout: 15_000,
    timeoutMsg: "did not navigate to /logs",
  });
  const viewer = await b.$('[data-testid="log-viewer"]');
  await viewer.waitForExist({ timeout: 15_000 });
  expect(await viewer.isExisting()).toBe(true);
}, 120_000);
