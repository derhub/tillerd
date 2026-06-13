import { remote } from "webdriverio";

export type Browser = Awaited<ReturnType<typeof remote>>;

const application = process.env.TILLERD_DESKTOP_BIN;

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
    async () => (await browser.$("body").getText()).includes("orchestrator: ready"),
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

// A fresh session opens with an empty leaf (ADR-0030: no auto-spawn). Click the empty leaf's
// "New terminal" picker to spawn a surface, then wait for its pane to mount. Returns the surface id.
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
