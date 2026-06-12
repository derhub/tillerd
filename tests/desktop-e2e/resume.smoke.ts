import { remote } from "webdriverio";

// Resume-after-restart: workspace state created in one app run survives a full restart. Two
// sequential WebDriver sessions launch the app twice against the SAME TILLERD_DIR (run.sh exports
// it for the whole run), so the second launch is a genuine restart that must rehydrate the first
// launch's project from workspace persistence (tillerd.db). Proves the persistence + adopt-or-spawn
// path the daemon-upgrade resume story (ADR-0029) relies on.

const application = process.env.TILLERD_DESKTOP_BIN;
if (!application) {
  throw new Error("set TILLERD_DESKTOP_BIN to the built desktop binary");
}

// Unique so the assertion is unambiguous even though TILLERD_DIR accumulates state across specs.
const PROJECT_NAME = `Resume ${Date.now()}`;

const capabilities = { "tauri:options": { application } } as Record<string, unknown>;

async function session<T>(run: (browser: WebdriverIO.Browser) => Promise<T>): Promise<T> {
  const browser = await remote({ hostname: "127.0.0.1", port: 4444, path: "/", capabilities });
  try {
    await browser.waitUntil(
      async () => (await browser.$("body").getText()).includes("orchestrator: ready"),
      { timeout: 45_000, timeoutMsg: "orchestrator did not reach ready" },
    );
    return await run(browser);
  } finally {
    // Ends the session, which closes the app — the next session is a true restart.
    await browser.deleteSession();
  }
}

// First launch: create the project (handleNewProject also makes a default session and navigates to
// it), then confirm it is in the sidebar before we tear the app down.
await session(async (browser) => {
  await browser.execute((name: string) => {
    (window as unknown as { prompt: (msg?: string) => string }).prompt = () => name;
  }, PROJECT_NAME);

  const newProject = await browser.$("button*=New project");
  await newProject.waitForExist({ timeout: 10_000 });
  await newProject.click();
  await browser.waitUntil(async () => (await browser.getUrl()).includes("/session/"), {
    timeout: 15_000,
    timeoutMsg: "creating a project did not produce a session route",
  });
  await browser.waitUntil(async () => (await browser.$("body").getText()).includes(PROJECT_NAME), {
    timeout: 10_000,
    timeoutMsg: "created project did not appear in the sidebar",
  });
});

// Second launch (restart): the project must reappear without recreating it.
await session(async (browser) => {
  await browser.waitUntil(async () => (await browser.$("body").getText()).includes(PROJECT_NAME), {
    timeout: 15_000,
    timeoutMsg: "project did not survive the restart — workspace persistence/resume regressed",
  });
});

console.log(`PASS: project "${PROJECT_NAME}" survived an app restart`);
