import { remote } from "webdriverio";

// UI-flow smoke for the workspace feature: create a project, then create a session within it.
// Asserts the create-project -> create-session-in-project path routes correctly and the project
// shows in the sidebar. Terminal rendering is covered by terminal.smoke.ts.

const application = process.env.TILLERD_DESKTOP_BIN;
if (!application) {
  throw new Error("set TILLERD_DESKTOP_BIN to the built desktop binary");
}

const PROJECT_NAME = "Smoke Project";

const browser = await remote({
  hostname: "127.0.0.1",
  port: 4444,
  path: "/",
  capabilities: { "tauri:options": { application } } as Record<string, unknown>,
});

async function dumpDom(): Promise<void> {
  try {
    const dom = await browser.execute(() => ({
      url: location.href,
      body: (document.body?.innerText ?? "").slice(0, 600),
      buttons: Array.from(document.querySelectorAll("button")).map((b) => ({
        text: b.textContent?.trim(),
        title: b.getAttribute("title"),
      })),
    }));
    console.error("DOM on failure:", JSON.stringify(dom));
  } catch {
    // best-effort diagnostics only
  }
}

try {
  // The workspace controls only act once the orchestrator is ready.
  await browser.waitUntil(
    async () => (await browser.$("body").getText()).includes("orchestrator: ready"),
    { timeout: 45_000, timeoutMsg: "orchestrator did not reach ready" },
  );

  // "New project" prompts for a name via window.prompt — stub it so the flow is non-blocking and
  // deterministic under WebDriver (native dialogs cannot be driven).
  await browser.execute((name: string) => {
    (window as unknown as { prompt: (msg?: string) => string }).prompt = () => name;
  }, PROJECT_NAME);

  // Create the project. handleNewProject creates the project + a default session and navigates to
  // that session, so reaching a /session/ route proves both the project and its first session.
  const newProject = await browser.$("button*=New project");
  await newProject.waitForExist({ timeout: 10_000 });
  await newProject.click();
  await browser.waitUntil(async () => (await browser.getUrl()).includes("/session/"), {
    timeout: 15_000,
    timeoutMsg: "creating a project did not produce a session route",
  });
  const firstSessionUrl = await browser.getUrl();

  // The project now appears in the sidebar with its own "New session" control.
  await browser.waitUntil(
    async () => (await browser.$("body").getText()).includes(PROJECT_NAME),
    { timeout: 10_000, timeoutMsg: "created project did not appear in the sidebar" },
  );

  // Create a second session WITHIN that project and assert it routes to a different session.
  const newSession = await browser.$(`button[title="New session in ${PROJECT_NAME}"]`);
  await newSession.waitForExist({ timeout: 10_000 });
  await newSession.click();
  await browser.waitUntil(
    async () => {
      const url = await browser.getUrl();
      return url.includes("/session/") && url !== firstSessionUrl;
    },
    {
      timeout: 15_000,
      timeoutMsg: "creating a session within the project did not route to a new session",
    },
  );

  console.log(`PASS: created project "${PROJECT_NAME}" and a session within it`);
} catch (err) {
  await dumpDom();
  throw err;
} finally {
  await browser.deleteSession();
}
