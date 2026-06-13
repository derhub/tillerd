import { afterEach, expect, test } from "bun:test";
import { type Browser, launchReadyApp } from "./helpers";

// The aggregate health indicator sits in the shell's bottom-right cluster. Clicking it opens a
// non-modal popover listing the orchestrator and each service, each with a logs link that performs
// a client-side navigation to the viewer filtered to that service. Popover open + the in-portal
// link navigation are real-webview concerns (happy-dom has no layout and no router), so they live
// here; row content and aggregate state are unit-tested.

let browser: Browser | undefined;
afterEach(async () => {
  await browser?.deleteSession();
  browser = undefined;
});

test("the health indicator lists each service and its logs link opens the filtered viewer", async () => {
  const b = await launchReadyApp();
  browser = b;

  await (await b.$('[aria-label^="Service health"]')).click();
  const panel = await b.$('[data-slot="popover-content"]');
  await panel.waitForExist({ timeout: 10_000 });

  const text = await panel.getText();
  expect(text).toContain("orchestrator");
  expect(text).toContain("gate");
  expect(text).toContain("daemon");

  // Click the gate row's logs link: it must client-side navigate (no hard `tauri://` load) to the
  // viewer pre-filtered to that service.
  const gateLogs = await b.$('a[href="/logs?service=tillerd-gate"]');
  await gateLogs.waitForExist({ timeout: 10_000 });
  await gateLogs.click();

  await b.waitUntil(async () => (await b.getUrl()).includes("service=tillerd-gate"), {
    timeout: 10_000,
    timeoutMsg: "logs link did not navigate to the filtered viewer",
  });
  // Wait for the viewer to mount: the route changes before React renders it, and on a slow
  // (xvfb) runner an immediate `isExisting` races the render.
  const viewer = await b.$('[data-testid="log-viewer"]');
  await viewer.waitForExist({ timeout: 10_000 });
  expect(await viewer.isExisting()).toBe(true);
  const facet = await b.$('select[aria-label="service"]');
  await facet.waitForExist({ timeout: 5_000 });
  expect(await facet.getValue()).toBe("tillerd-gate");
}, 120_000);
