import { readdirSync, unlinkSync } from "node:fs";
import { join } from "node:path";

import { remote } from "webdriverio";

export type Browser = Awaited<ReturnType<typeof remote>>;

const application = process.env.TILLERD_DESKTOP_BIN;

// Specs share one TILLERD_DIR/logs across the run; remove other specs' seed files so they don't
// bury this spec's seeded rows in the merged, timestamp-ordered view.
export function clearLogSeeds(logsDir: string): void {
  try {
    for (const f of readdirSync(logsDir)) {
      if (f.startsWith("zzz-e2e-") && f.endsWith(".log")) unlinkSync(join(logsDir, f));
    }
  } catch {
    // logs dir may not exist yet — nothing to clear
  }
}

// Launch the desktop app through tauri-webdriver and wait for the embedded orchestrator to reach
// ready. Called from inside a test so each app launch is DEFERRED to when its test runs; bun runs
// tests sequentially, so only one app is live at a time (one tauri-webdriver, one socket).
export async function launchReadyApp(): Promise<Browser> {
  if (!application) {
    throw new Error("set TILLERD_DESKTOP_BIN to the built desktop binary");
  }
  const browser = await remote({
    hostname: "127.0.0.1",
    port: 4444,
    path: "/",
    capabilities: { "tauri:options": { application } } as Record<string, unknown>,
    logLevel: "error",
  });
  await browser.waitUntil(
    async () => (await browser.$("body").getText()).includes("services: ready"),
    { timeout: 45_000, timeoutMsg: "orchestrator did not reach ready" },
  );
  return browser;
}

// Native `window.prompt` (used by "New project") cannot be driven under WebDriver — stub it.
export async function stubPrompt(browser: Browser, value: string): Promise<void> {
  await browser.execute((name: string) => {
    (window as unknown as { prompt: (msg?: string) => string }).prompt = () => name;
  }, value);
}

// Create a project (which also makes a default session and navigates to it) and return the
// resulting session route URL.
export async function createProject(browser: Browser, name: string): Promise<string> {
  await stubPrompt(browser, name);
  const button = await browser.$("button*=New project");
  await button.waitForExist({ timeout: 10_000 });
  await button.click();
  await browser.waitUntil(async () => (await browser.getUrl()).includes("/session/"), {
    timeout: 15_000,
    timeoutMsg: "creating a project did not produce a session route",
  });
  return browser.getUrl();
}

// The currently-mounted terminal pane's surface id, or "" before one exists.
export async function surfaceId(browser: Browser): Promise<string> {
  const el = await browser.$("[data-surface-id]");
  if (!(await el.isExisting())) return "";
  return (await el.getAttribute("data-surface-id")) ?? "";
}

// Fresh session has an empty leaf (no auto-spawn); click "New terminal" to spawn, return its id.
export async function openTerminal(browser: Browser): Promise<string> {
  const spawn = await browser.$("button*=New terminal");
  await spawn.waitForExist({ timeout: 15_000 });
  await spawn.click();
  await browser.waitUntil(async () => (await surfaceId(browser)).length > 0, {
    timeout: 20_000,
    timeoutMsg: "terminal did not mount a surface after spawn",
  });
  return surfaceId(browser);
}

// Navigate to the log viewer the way the app does at runtime — a client-side route change, no
// reload. The native View > Logs menu is not WebDriver-accessible; a hard URL load is unreliable
// (custom-scheme opaque origin + no SPA fallback); and an injected `import()` of the Tauri API
// can't resolve a bare specifier. So push the route and fire `popstate`, which react-router's
// browser history listens for — pure sync DOM, no Promise to serialize.
export async function openLogViewer(browser: Browser): Promise<void> {
  await browser.execute(() => {
    window.history.pushState({}, "", "/logs");
    window.dispatchEvent(new Event("popstate"));
  });
  await browser.waitUntil(async () => (await browser.getUrl()).includes("/logs"), {
    timeout: 15_000,
    timeoutMsg: "client navigation did not route to /logs",
  });
  const viewer = await browser.$('[data-testid="log-viewer"]');
  await viewer.waitForExist({ timeout: 15_000 });
}

// Monotonic counter keeps names unique for back-to-back tests sharing one app (no boot to space
// out Date.now()).
let nameSeq = 0;
export function uniqueName(prefix: string): string {
  return `${prefix} ${Date.now()}-${nameSeq++}`;
}

export async function resetToHome(browser: Browser): Promise<void> {
  // Escape closes the focused palette/menu/dialog/rename input; body mousedown closes the rest.
  await browser.keys(["Escape"]);
  await browser.execute(() => {
    document.body.dispatchEvent(new MouseEvent("mousedown", { bubbles: true }));
  });
  await browser.execute(() => {
    window.history.pushState({}, "", "/");
    window.dispatchEvent(new Event("popstate"));
  });
  await browser.waitUntil(
    async () => {
      for (const sel of [
        '[data-testid="command-center"]',
        '[role="menu"]',
        '[role="alertdialog"]',
        '[data-testid="inline-rename-input"]',
      ]) {
        if (await (await browser.$(sel)).isExisting()) return false;
      }
      return (await browser.$("button*=New project")).isExisting();
    },
    { timeout: 10_000, timeoutMsg: "resetToHome did not reach a clean home baseline" },
  );
}
