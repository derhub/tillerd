import { afterEach, expect, test } from "bun:test";
import { type Browser, launchReadyApp } from "./helpers";

// The log viewer is reachable two ways: the sidebar "Logs" nav, and the native
// View > Logs menu (which emits "menu:navigate"). Both route to /logs and render
// the viewer. The native menu click itself is not WebDriver-accessible, so the
// menu path is exercised through the event its handler emits.

let browser: Browser | undefined;
afterEach(async () => {
  await browser?.deleteSession();
  browser = undefined;
});

async function expectViewerAtLogs(b: Browser): Promise<void> {
  await b.waitUntil(async () => (await b.getUrl()).includes("/logs"), {
    timeout: 10_000,
    timeoutMsg: "did not route to /logs",
  });
  const viewer = await b.$('[data-testid="log-viewer"]');
  await viewer.waitForExist({ timeout: 10_000 });
  expect(await viewer.isExisting()).toBe(true);
}

test("the sidebar Logs nav opens the log viewer", async () => {
  browser = await launchReadyApp();
  const logs = await browser.$("a*=Logs");
  await logs.waitForExist({ timeout: 10_000 });
  await logs.click();
  await expectViewerAtLogs(browser);
}, 120_000);

test("the menu:navigate event routes to the log viewer", async () => {
  browser = await launchReadyApp();
  await browser.execute(async () => {
    const { emit } = await import("@tauri-apps/api/event");
    await emit("menu:navigate", "/logs");
  });
  await expectViewerAtLogs(browser);
}, 120_000);
