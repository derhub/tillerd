import { afterEach, expect, test } from "bun:test";
import { type Browser, createProject, launchReadyApp } from "./helpers";

// UI-flow for the workspace feature: create a project, then a second session within it. Asserts the
// create-project -> create-session-in-project path routes correctly and the project shows in the
// sidebar. Terminal rendering is covered by terminal.test.ts.

let browser: Browser | undefined;
afterEach(async () => {
  await browser?.deleteSession();
  browser = undefined;
});

test("creates a project and a session within it", async () => {
  const b = (browser = await launchReadyApp());
  const project = `Smoke ${Date.now()}`;

  // Creating the project also makes a default session and navigates to it.
  const firstSessionUrl = await createProject(b, project);
  expect(firstSessionUrl).toContain("/session/");

  // The project appears in the sidebar with its own "New session" control.
  await b.waitUntil(async () => (await b.$("body").getText()).includes(project), {
    timeout: 10_000,
    timeoutMsg: "created project did not appear in the sidebar",
  });

  // A second session within that project routes to a different session.
  const newSession = await b.$(`button[title="New session in ${project}"]`);
  await newSession.waitForExist({ timeout: 10_000 });
  await newSession.click();
  await b.waitUntil(
    async () => {
      const url = await b.getUrl();
      return url.includes("/session/") && url !== firstSessionUrl;
    },
    { timeout: 15_000, timeoutMsg: "creating a session within the project did not route anew" },
  );

  expect(await b.getUrl()).not.toBe(firstSessionUrl);
}, 120_000);
