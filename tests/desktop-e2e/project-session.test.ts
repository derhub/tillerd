import { expect, test } from "bun:test";
import { createProject } from "./helpers";
import { getApp } from "./shared-app";

// Test create-project -> create-session-in-project routing and sidebar display.

test("creates a project and a session within it", async () => {
  const b = getApp();
  const project = `Smoke ${Date.now()}`;

  const firstSessionUrl = await createProject(b, project);
  expect(firstSessionUrl).toContain("/session/");

  await b.waitUntil(async () => (await b.$("body").getText()).includes(project), {
    timeout: 10_000,
    timeoutMsg: "created project did not appear in the sidebar",
  });

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
