import { expect, test } from "bun:test";

import { type Browser } from "./helpers";
import { getApp } from "./shared-app";

// The aggregate health indicator sits in the shell's bottom-right cluster. Clicking it opens a
// non-modal popover listing the orchestrator and each service; each row's logs control opens the
// bottom panel's Logs tab filtered to that service (spec: "the tab via the health panel's logs
// link"). Popover open, bottom-panel open, and the resulting service-filtered rows are
// real-webview concerns (happy-dom has no layout and no router), so they live here; row content
// and aggregate state are unit-tested. The /logs route's own `?service=` deep link (the other
// host the spec requires) is exercised separately below, independent of the health panel.

// The row's accessible label uses the same short display name as its rendered text -- the
// orchestrator row is labelled "orchestrator" even though its logs service is "tillerd-desktop".
function displayName(service: string): string {
  return service === "tillerd-desktop" ? "orchestrator" : service.replace(/^tillerd-/, "");
}

// Open the popover, click `service`'s logs control, and assert every rendered row is that service.
async function expectLogsFilteredTo(b: Browser, service: string) {
  await (await b.$('[aria-label^="Service health"]')).click();
  await (await b.$('[data-slot="popover-content"]')).waitForExist({ timeout: 10_000 });
  await (await b.$(`[aria-label="Show logs for ${displayName(service)}"]`)).click();

  await (await b.$('[data-testid="log-viewer"]')).waitForExist({ timeout: 10_000 });
  // Wait for only this service's rows to be present, to avoid stale rows from the prior filter.
  let rows: (string | null)[] = [];
  await b.waitUntil(
    async () => {
      const selected = await (await b.$('select[aria-label="service"]')).getValue();
      if (selected !== service) return false;
      rows = await b.execute(() =>
        Array.from(document.querySelectorAll("[data-service]")).map((el) =>
          el.getAttribute("data-service"),
        ),
      );
      return rows.every((s) => s === service);
    },
    { timeout: 15_000, timeoutMsg: `logs list was not filtered to ${service}` },
  );
  expect(rows.every((s) => s === service)).toBe(true);
}

test("each health-row logs control opens the bottom panel filtered to its own service", async () => {
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

  // The bottom panel's open state is a persisted workbench setting -- close it via the
  // title-bar toggle so a later (own-launch) app boot does not start with it already open
  // (see notification-center.test.ts, which does the same for its own bell-triggered open).
  await (await b.$('[aria-label="Toggle bottom panel"]')).click();
  await (await b.$('[data-testid="log-viewer"]')).waitForExist({ timeout: 10_000, reverse: true });
}, 120_000);

// The route itself must keep honoring `?service=` as a plain deep link (spec: "Both hosts SHALL
// honor the service filter"), independent of the health panel's bottom-panel affordance above.
test("the /logs route's own service filter still works as a deep link", async () => {
  const b = getApp();

  await b.execute(() => {
    window.history.pushState({}, "", "/logs?service=tillerd-desktop");
    window.dispatchEvent(new Event("popstate"));
  });
  await b.waitUntil(async () => (await b.getUrl()).includes("service=tillerd-desktop"), {
    timeout: 10_000,
    timeoutMsg: "client navigation did not route to /logs?service=",
  });
  await (await b.$('[data-testid="log-viewer"]')).waitForExist({ timeout: 15_000 });
  await b.waitUntil(
    async () => {
      const rows = await b.execute(() =>
        Array.from(document.querySelectorAll("[data-service]")).map((el) =>
          el.getAttribute("data-service"),
        ),
      );
      return rows.length > 0 && rows.every((s) => s === "tillerd-desktop");
    },
    { timeout: 15_000, timeoutMsg: "logs list was not filtered to tillerd-desktop via route" },
  );
}, 120_000);
