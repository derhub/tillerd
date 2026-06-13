import { afterEach, expect, test } from "bun:test";
import { type Browser, launchReadyApp } from "./helpers";

// The log viewer is reachable from the sidebar "Logs" nav and the native View > Logs
// menu (which emits "menu:navigate"). The native menu is not WebDriver-accessible, and
// emitting the event from the webview is not driveable via tauri-webdriver's sync execute,
// so this covers the DOM-reachable path: the sidebar nav routes to /logs and renders the
// viewer. The menu's renderer handler is exercised at the unit/manual layer.

let browser: Browser | undefined;
afterEach(async () => {
  await browser?.deleteSession();
  browser = undefined;
});

test("the sidebar Logs nav opens the log viewer", async () => {
  const b = await launchReadyApp();
  browser = b;

  const logs = await b.$("a*=Logs");
  await logs.waitForExist({ timeout: 10_000 });
  await logs.click();

  await b.waitUntil(async () => (await b.getUrl()).includes("/logs"), {
    timeout: 10_000,
    timeoutMsg: "Logs nav did not route to /logs",
  });
  const viewer = await b.$('[data-testid="log-viewer"]');
  await viewer.waitForExist({ timeout: 10_000 });
  expect(await viewer.isExisting()).toBe(true);
}, 120_000);
