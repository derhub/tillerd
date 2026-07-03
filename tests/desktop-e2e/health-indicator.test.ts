import { expect, test } from "bun:test";

import { type Browser } from "./helpers";
import { getApp } from "./shared-app";

// The aggregate health indicator sits in the shell's bottom-right cluster. Clicking it opens a
// non-modal popover listing the orchestrator and each service, each with a logs link that performs
// a client-side navigation to the viewer filtered to that service. Popover open + the in-portal
// link navigation are real-webview concerns (happy-dom has no layout and no router), so they live
// here; row content and aggregate state are unit-tested.

// Open the popover, click `service`'s logs link, and assert every rendered row is that service.
async function expectLogsFilteredTo(b: Browser, service: string) {
  await (await b.$('[aria-label^="Service health"]')).click();
  await (await b.$('[data-slot="popover-content"]')).waitForExist({ timeout: 10_000 });
  await (await b.$(`a[href="/logs?service=${service}"]`)).click();

  await b.waitUntil(async () => (await b.getUrl()).includes(`service=${service}`), {
    timeout: 10_000,
    timeoutMsg: `${service} logs link did not navigate`,
  });
  await (await b.$('[data-testid="log-viewer"]')).waitForExist({ timeout: 10_000 });
  // Wait for this service's rows to avoid stale rows from the prior navigation.
  await b.waitUntil(
    () => b.execute((s) => document.querySelectorAll(`[data-service="${s}"]`).length > 0, service),
    { timeout: 15_000, timeoutMsg: `no ${service} rows appeared` },
  );
  const rows = await b.execute(() =>
    Array.from(document.querySelectorAll("[data-service]")).map((el) =>
      el.getAttribute("data-service"),
    ),
  );
  expect(rows.length).toBeGreaterThan(0);
  expect(rows.every((s) => s === service)).toBe(true);
}

test("each health-row logs link filters the viewer to its own service", async () => {
  const b = getApp();

  await (await b.$('[aria-label^="Service health"]')).click();
  const panel = await b.$('[data-slot="popover-content"]');
  await panel.waitForExist({ timeout: 10_000 });
  const panelText = await panel.getText();
  expect(panelText).toContain("orchestrator");
  expect(panelText).toContain("gate");
  expect(panelText).toContain("daemon");
  await b.keys(["Escape"]); // close it; each assertion reopens the popover

  await expectLogsFilteredTo(b, "tillerd-gate");
  await expectLogsFilteredTo(b, "tillerd-desktop");
}, 120_000);
