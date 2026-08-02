import { test } from "bun:test";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { clearLogSeeds, openLogViewer, type Browser } from "./helpers";
import { getApp } from "./shared-app";

// The viewer's facets are exact-match. Each test seeds a log file (far-future timestamps so rows
// sort to the bottom and auto-scroll into view), changes one facet, and asserts the rendered rows.

const SCROLL = '[data-testid="log-scroll"]';
const LEVEL_SELECT = 'select[aria-label="Level"]';
const SERVICE_SELECT = 'select[aria-label="service"]';

// Route cleanup can briefly replace the viewer. Query on every poll so a detached WebDriver
// element means "not ready" rather than aborting the assertion.
async function scrollText(b: Browser): Promise<string> {
  const scroll = await b.$(SCROLL);
  return (await scroll.isExisting()) ? scroll.getText() : "";
}

function line(level: string, msg: string, i: number, service = "e2e"): string {
  return `${JSON.stringify({
    timestamp: `2099-01-01T00:00:00.${String(i).padStart(6, "0")}Z`,
    level,
    fields: { message: msg },
    spans: [{ "service.name": service, name: "service" }],
  })}\n`;
}

// The facet <select> is React-controlled, so set the value and dispatch a bubbling `change` to run
// onChange (selectByAttribute does not reliably trigger it).
async function setSelectValue(b: Browser, selector: string, value: string): Promise<void> {
  await b.execute(
    (sel: string, val: string) => {
      const doc = globalThis as unknown as {
        document: {
          querySelector(s: string): { value: string; dispatchEvent(e: Event): boolean } | null;
        };
      };
      const el = doc.document.querySelector(sel);
      if (el) {
        el.value = val;
        el.dispatchEvent(new Event("change", { bubbles: true }));
      }
    },
    selector,
    value,
  );
}

test("the level filter shows only the chosen level", async () => {
  const dir = join(process.env.TILLERD_DIR ?? `${process.env.HOME}/.tillerd`, "logs");
  mkdirSync(dir, { recursive: true });
  clearLogSeeds(dir);
  const seed = [
    ...Array.from({ length: 8 }, (_, i) => line("ERROR", `errline ${i}`, i)),
    ...Array.from({ length: 8 }, (_, i) => line("INFO", `infoline ${i}`, 8 + i)),
  ].join("");
  writeFileSync(join(dir, "zzz-e2e-filter.log"), seed);

  const b = getApp();
  await openLogViewer(b);
  await (await b.$(SCROLL)).waitForExist({ timeout: 15_000 });
  await setSelectValue(b, LEVEL_SELECT, "");
  await setSelectValue(b, SERVICE_SELECT, "");

  const text = (): Promise<string> => scrollText(b);

  // Unfiltered, auto-scrolled to the bottom: the latest rows (INFO) are visible.
  await b.waitUntil(async () => (await text()).includes("infoline"), {
    timeout: 15_000,
    timeoutMsg: "seeded logs did not appear",
  });

  // Choose ERROR -> only ERROR rows remain (INFO leaves the data, so it never renders).
  await setSelectValue(b, LEVEL_SELECT, "ERROR");
  await b.waitUntil(
    async () => {
      const t = await text();
      return t.includes("errline") && !t.includes("infoline");
    },
    { timeout: 10_000, timeoutMsg: "level filter did not restrict to ERROR only" },
  );
}, 180_000);

test("the service filter shows only the chosen service", async () => {
  const dir = join(process.env.TILLERD_DIR ?? `${process.env.HOME}/.tillerd`, "logs");
  mkdirSync(dir, { recursive: true });
  clearLogSeeds(dir);
  // Beta sorts below alpha, so the bottom-pinned view shows beta first; filtering to alpha must
  // bring alpha in and drop beta.
  const seed = [
    ...Array.from({ length: 8 }, (_, i) => line("INFO", `alphaline ${i}`, i, "svc-alpha")),
    ...Array.from({ length: 8 }, (_, i) => line("INFO", `betaline ${i}`, 8 + i, "svc-beta")),
  ].join("");
  writeFileSync(join(dir, "zzz-e2e-service.log"), seed);

  const b = getApp();
  await openLogViewer(b);
  await (await b.$(SCROLL)).waitForExist({ timeout: 15_000 });
  await setSelectValue(b, LEVEL_SELECT, "");
  await setSelectValue(b, SERVICE_SELECT, "");

  const text = (): Promise<string> => scrollText(b);

  await b.waitUntil(async () => (await text()).includes("betaline"), {
    timeout: 15_000,
    timeoutMsg: "seeded service logs did not appear",
  });

  await setSelectValue(b, SERVICE_SELECT, "svc-alpha");
  await b.waitUntil(
    async () => {
      const t = await text();
      return t.includes("alphaline") && !t.includes("betaline");
    },
    { timeout: 10_000, timeoutMsg: "service filter did not restrict to svc-alpha only" },
  );
}, 180_000);
