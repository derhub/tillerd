import { remote } from "webdriverio";

// Surface isolation: each session drives its OWN terminal surface. The terminal pane exposes its
// surface id as `data-surface-id`; we create two sessions in the same project and assert they mount
// two DISTINCT surfaces. If they shared one, the second session would reuse the first's id. The live
// counterpart of the data-layer test `each_session_gets_its_own_distinct_surface`.

const application = process.env.TILLERD_DESKTOP_BIN;
if (!application) {
  throw new Error("set TILLERD_DESKTOP_BIN to the built desktop binary");
}

const PROJECT = `Isolation ${Date.now()}`;

const browser = await remote({
  hostname: "127.0.0.1",
  port: 4444,
  path: "/",
  capabilities: { "tauri:options": { application } } as Record<string, unknown>,
});

// The active terminal pane's surface id, or "" before it has mounted/created one.
async function surfaceId(): Promise<string> {
  const el = await browser.$("[data-surface-id]");
  if (!(await el.isExisting())) return "";
  return (await el.getAttribute("data-surface-id")) ?? "";
}

try {
  await browser.waitUntil(
    async () => (await browser.$("body").getText()).includes("orchestrator: ready"),
    { timeout: 45_000, timeoutMsg: "orchestrator did not reach ready" },
  );

  // Session 1: create the project (also makes a default session) and capture its surface id.
  await browser.execute((name: string) => {
    (window as unknown as { prompt: (msg?: string) => string }).prompt = () => name;
  }, PROJECT);
  const newProject = await browser.$("button*=New project");
  await newProject.waitForExist({ timeout: 10_000 });
  await newProject.click();
  await browser.waitUntil(async () => (await browser.getUrl()).includes("/session/"), {
    timeout: 15_000,
    timeoutMsg: "creating a project did not produce a session route",
  });
  const session1Url = await browser.getUrl();
  await browser.waitUntil(async () => (await surfaceId()).length > 0, {
    timeout: 20_000,
    timeoutMsg: "session 1 terminal never mounted a surface",
  });
  const s1 = await surfaceId();

  // Session 2: a new session within the same project must mount its OWN distinct surface.
  const newSession = await browser.$(`button[title="New session in ${PROJECT}"]`);
  await newSession.waitForExist({ timeout: 10_000 });
  await newSession.click();
  await browser.waitUntil(
    async () => {
      const url = await browser.getUrl();
      return url.includes("/session/") && url !== session1Url;
    },
    { timeout: 15_000, timeoutMsg: "second session did not route to its own surface" },
  );
  // Wait for a mounted surface whose id differs from session 1's. A shared surface would keep id
  // == s1 and time out here — which is exactly the isolation failure we want to catch.
  await browser.waitUntil(
    async () => {
      const id = await surfaceId();
      return id.length > 0 && id !== s1;
    },
    {
      timeout: 20_000,
      timeoutMsg: "session 2 did not mount a surface distinct from session 1's",
    },
  );
  const s2 = await surfaceId();

  if (s1 === s2) {
    throw new Error(`both sessions share surface ${s1} — surfaces are not isolated`);
  }

  console.log(`PASS: session 1 surface ${s1} != session 2 surface ${s2} — each session owns its surface`);
} finally {
  await browser.deleteSession();
}
