import { afterEach, test } from "bun:test";
import { appendFileSync, mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { type Browser, clearLogSeeds, launchReadyApp, openLogViewer } from "./helpers";

// Auto-scroll needs real layout (scrollHeight/clientHeight), which happy-dom lacks — so it is
// covered here, in a real webview. Seeds an overflowing log file (far-future timestamps so
// appended lines sort to the bottom) and asserts the view follows new logs to the bottom.
//
// Scope note: only the "follow to bottom" behavior is asserted. Pause-on-scroll-up /
// resume-at-bottom is racy to drive deterministically (WebDriver's synthetic scrollTop set vs the
// async onScroll flag vs the 1s live-log poll) and is verified manually; the feature itself is the
// standard stick-to-bottom ref pattern in LogViewer.

const SCROLL = '[data-testid="log-scroll"]';

interface Metrics {
  height: number;
  top: number;
  client: number;
}

function line(i: number): string {
  return `${JSON.stringify({
    timestamp: `2099-01-01T00:00:00.${String(i).padStart(6, "0")}Z`,
    level: "INFO",
    fields: { message: `seed ${i}` },
    spans: [{ "service.name": "e2e", name: "service" }],
  })}\n`;
}

function logFile(): string {
  const dir = join(process.env.TILLERD_DIR ?? `${process.env.HOME}/.tillerd`, "logs");
  mkdirSync(dir, { recursive: true });
  clearLogSeeds(dir);
  return join(dir, "zzz-e2e-scroll.log");
}

function readMetrics(b: Browser): Promise<Metrics> {
  return b.execute((sel: string) => {
    const doc = globalThis as unknown as {
      document: {
        querySelector(
          s: string,
        ): { scrollTop: number; scrollHeight: number; clientHeight: number } | null;
      };
    };
    const el = doc.document.querySelector(sel);
    return el
      ? { height: el.scrollHeight, top: el.scrollTop, client: el.clientHeight }
      : { height: 0, top: 0, client: 0 };
  }, SCROLL) as Promise<Metrics>;
}

const atBottom = (m: Metrics): boolean => m.height - m.top - m.client < 16;

let browser: Browser | undefined;
afterEach(async () => {
  await browser?.deleteSession();
  browser = undefined;
});

test("auto-scroll follows new logs to the bottom", async () => {
  const file = logFile();
  writeFileSync(file, Array.from({ length: 800 }, (_, i) => line(i)).join(""));

  const b = await launchReadyApp();
  browser = b;
  await openLogViewer(b);
  await (await b.$(SCROLL)).waitForExist({ timeout: 15_000 });

  // Overflowing content auto-scrolls to the bottom on open.
  await b.waitUntil(async () => atBottom(await readMetrics(b)), {
    timeout: 15_000,
    timeoutMsg: "did not auto-scroll to the bottom on load",
  });

  // Appended lines (later timestamps) are followed to the new bottom.
  for (let i = 800; i < 900; i++) appendFileSync(file, line(i));
  await b.waitUntil(
    async () => {
      const text = await (await b.$(SCROLL)).getText();
      return atBottom(await readMetrics(b)) && text.includes("seed 899");
    },
    { timeout: 10_000, timeoutMsg: "did not follow appended logs to the bottom" },
  );
}, 180_000);
