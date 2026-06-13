import { afterEach, expect, test } from "bun:test";
import { type Browser, launchReadyApp, openLogViewer } from "./helpers";

// The log viewer is reached via the native View > Logs menu (emits menu:navigate). The native
// menu is not WebDriver-accessible, so the e2e drives the same menu:navigate event and asserts
// the viewer renders.

let browser: Browser | undefined;
afterEach(async () => {
  await browser?.deleteSession();
  browser = undefined;
});

test("the View > Logs menu routes to the log viewer", async () => {
  const b = await launchReadyApp();
  browser = b;
  await openLogViewer(b);
  expect(await (await b.$('[data-testid="log-viewer"]')).isExisting()).toBe(true);
}, 120_000);
