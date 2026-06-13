import { afterEach, expect, test } from "bun:test";
import { appendFileSync, mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { type Browser, launchReadyApp } from "./helpers";

// Auto-scroll needs real layout (scrollHeight/clientHeight), which happy-dom lacks — so it is
// covered here, in a real webview. The test seeds a log file the viewer tails (far-future
// timestamps so appended lines sort to the bottom), then drives the scroll container directly.

const SCROLL = '[data-testid="log-scroll"]';

interface Metrics {
  top: number;
  height: number;
  client: number;
}

function line(i: number): string {
  const ts = `2099-01-01T00:00:00.${String(i).padStart(6, "0")}Z`;
  return `${JSON.stringify({
    timestamp: ts,
    level: "INFO",
    fields: { message: `seed ${i}` },
    spans: [{ "service.name": "e2e", name: "service" }],
  })}\n`;
}

function logFile(): string {
  const dir = join(process.env.TILLERD_DIR ?? `${process.env.HOME}/.tillerd`, "logs");
  mkdirSync(dir, { recursive: true });
  return join(dir, "zzz-e2e-scroll.log");
}

// The DOM element exposes scrollTop/scrollHeight/clientHeight; map them in-page to {top,height,client}.
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
      ? { top: el.scrollTop, height: el.scrollHeight, client: el.clientHeight }
      : { top: 0, height: 0, client: 0 };
  }, SCROLL) as Promise<Metrics>;
}

function setScrollTop(b: Browser, value: number): Promise<unknown> {
  return b.execute(
    (sel: string, v: number) => {
      const doc = globalThis as unknown as {
        document: { querySelector(s: string): { scrollTop: number; scrollHeight: number } | null };
      };
      const el = doc.document.querySelector(sel);
      if (el) el.scrollTop = v === -1 ? el.scrollHeight : v;
    },
    SCROLL,
    value,
  );
}

const atBottom = (m: Metrics): boolean => m.height - m.top - m.client < 16;

let browser: Browser | undefined;
afterEach(async () => {
  await browser?.deleteSession();
  browser = undefined;
});

test("auto-scroll follows new logs, pauses on scroll-up, resumes at bottom", async () => {
  const file = logFile();
  writeFileSync(file, Array.from({ length: 800 }, (_, i) => line(i)).join(""));

  const b = await launchReadyApp();
  browser = b;
  const origin = new URL(await b.getUrl()).origin;
  await b.url(`${origin}/logs`);
  await (await b.$(SCROLL)).waitForExist({ timeout: 15_000 });

  // Overflowing content + initial stick → pinned to the bottom.
  await b.waitUntil(async () => atBottom(await readMetrics(b)), {
    timeout: 15_000,
    timeoutMsg: "did not auto-scroll to the bottom on load",
  });

  // Scroll up → auto-scroll pauses: appended lines must NOT yank back to the bottom.
  await setScrollTop(b, 0);
  for (let i = 800; i < 900; i++) appendFileSync(file, line(i));
  await b.pause(1500); // past the 1s poll interval
  expect(atBottom(await readMetrics(b))).toBe(false);

  // Scroll back to the bottom → auto-scroll resumes.
  await setScrollTop(b, -1);
  await b.waitUntil(async () => atBottom(await readMetrics(b)), { timeout: 5_000 });
  for (let i = 900; i < 1000; i++) appendFileSync(file, line(i));
  await b.waitUntil(async () => atBottom(await readMetrics(b)), {
    timeout: 5_000,
    timeoutMsg: "did not resume auto-scroll after returning to the bottom",
  });
}, 180_000);
