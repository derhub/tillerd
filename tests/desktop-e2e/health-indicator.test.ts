import { afterEach, expect, test } from "bun:test";
import { type Browser, launchReadyApp } from "./helpers";

// The aggregate health indicator sits in the shell's bottom-right cluster. Clicking it opens a
// non-modal popover listing the orchestrator and each service, with a logs link filtered to that
// service. Popover open + panel layout are real-webview concerns (happy-dom has no layout), so they
// live here; the row content and the aggregate state are unit-tested.

let browser: Browser | undefined;
afterEach(async () => {
  await browser?.deleteSession();
  browser = undefined;
});

test("the health indicator opens a panel listing each service with a filtered logs link", async () => {
  const b = await launchReadyApp();
  browser = b;

  const trigger = await b.$('[aria-label^="Service health"]');
  await trigger.waitForExist({ timeout: 15_000 });
  await trigger.click();

  const panel = await b.$('[data-slot="popover-content"]');
  await panel.waitForExist({ timeout: 10_000 });

  const text = await panel.getText();
  expect(text).toContain("orchestrator");
  expect(text).toContain("gate");
  expect(text).toContain("daemon");

  // A row links to that service's logs, pre-filtered.
  const gateLogs = await b.$('a[href="/logs?service=tillerd-gate"]');
  expect(await gateLogs.isExisting()).toBe(true);
}, 120_000);
