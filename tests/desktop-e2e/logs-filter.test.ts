import { afterEach, test } from "bun:test";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { type Browser, launchReadyApp } from "./helpers";

// The level filter is exact-match: choosing a level shows only records of that level. Seeds a log
// file with distinct ERROR/INFO messages (far-future timestamps so they sort to the bottom) and
// asserts the rendered rows after selecting ERROR.

const SCROLL = '[data-testid="log-scroll"]';
const LEVEL_SELECT = 'select[aria-label="Level"]';

function line(level: string, msg: string, i: number): string {
  return `${JSON.stringify({
    timestamp: `2099-01-01T00:00:00.${String(i).padStart(6, "0")}Z`,
    level,
    fields: { message: msg },
    spans: [{ "service.name": "e2e", name: "service" }],
  })}\n`;
}

let browser: Browser | undefined;
afterEach(async () => {
  await browser?.deleteSession();
  browser = undefined;
});

test("the level filter shows only the chosen level", async () => {
  const dir = join(process.env.TILLERD_DIR ?? `${process.env.HOME}/.tillerd`, "logs");
  mkdirSync(dir, { recursive: true });
  const seed = [
    ...Array.from({ length: 8 }, (_, i) => line("ERROR", `errline ${i}`, i)),
    ...Array.from({ length: 8 }, (_, i) => line("INFO", `infoline ${i}`, 8 + i)),
  ].join("");
  writeFileSync(join(dir, "zzz-e2e-filter.log"), seed);

  const b = await launchReadyApp();
  browser = b;
  const origin = new URL(await b.getUrl()).origin;
  await b.url(`${origin}/logs`);
  await (await b.$(SCROLL)).waitForExist({ timeout: 15_000 });

  const text = async (): Promise<string> => (await b.$(SCROLL)).getText();

  // Unfiltered, auto-scrolled to the bottom: the latest rows (INFO) are visible.
  await b.waitUntil(async () => (await text()).includes("infoline"), {
    timeout: 15_000,
    timeoutMsg: "seeded logs did not appear",
  });

  // Choose ERROR → only ERROR rows remain (INFO leaves the data, so it never renders).
  await (await b.$(LEVEL_SELECT)).selectByAttribute("value", "ERROR");
  await b.waitUntil(
    async () => {
      const t = await text();
      return t.includes("errline") && !t.includes("infoline");
    },
    { timeout: 10_000, timeoutMsg: "level filter did not restrict to ERROR only" },
  );
}, 180_000);
