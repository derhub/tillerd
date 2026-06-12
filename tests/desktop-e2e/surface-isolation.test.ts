import { afterEach, expect, test } from "bun:test";
import { type Browser, createProject, launchReadyApp } from "./helpers";

// Surface isolation: each session drives its OWN terminal surface. The pane exposes its surface id
// as `data-surface-id`; two sessions in the same project must mount two DISTINCT surfaces. A shared
// surface would reuse the first id. The live counterpart of the data-layer test
// `each_session_gets_its_own_distinct_surface`.

let browser: Browser | undefined;
afterEach(async () => {
  await browser?.deleteSession();
  browser = undefined;
});

test("each session mounts its own distinct surface", async () => {
  const b = (browser = await launchReadyApp());
  const project = `Isolation ${Date.now()}`;

  // The active terminal pane's surface id, or "" before it has mounted/created one.
  const surfaceId = async (): Promise<string> => {
    const el = await b.$("[data-surface-id]");
    if (!(await el.isExisting())) return "";
    return (await el.getAttribute("data-surface-id")) ?? "";
  };

  const session1Url = await createProject(b, project);
  await b.waitUntil(async () => (await surfaceId()).length > 0, {
    timeout: 20_000,
    timeoutMsg: "session 1 terminal never mounted a surface",
  });
  const s1 = await surfaceId();

  const newSession = await b.$(`button[title="New session in ${project}"]`);
  await newSession.waitForExist({ timeout: 10_000 });
  await newSession.click();
  await b.waitUntil(
    async () => {
      const url = await b.getUrl();
      return url.includes("/session/") && url !== session1Url;
    },
    { timeout: 15_000, timeoutMsg: "second session did not route to its own surface" },
  );
  // A shared surface would keep id == s1 and time out here — the isolation failure we want to catch.
  await b.waitUntil(
    async () => {
      const id = await surfaceId();
      return id.length > 0 && id !== s1;
    },
    { timeout: 20_000, timeoutMsg: "session 2 did not mount a surface distinct from session 1's" },
  );
  const s2 = await surfaceId();

  expect(s1).toBeTruthy();
  expect(s2).toBeTruthy();
  expect(s2).not.toBe(s1);
}, 120_000);
