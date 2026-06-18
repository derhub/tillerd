import { expect, test } from "bun:test";
import { createProject, resetToHome, stubPrompt, uniqueName, type Browser } from "./helpers";
import { getApp } from "./shared-app";

// The workspace switcher is the left-rail strip (data-testid="workspace-switcher"); each row is a
// "workspace-item" button carrying its id. Native windows are unreachable under WebDriver, so detach
// is asserted via its DOM affordance (the detached indicator), not by driving a second window.

async function workspaceItem(b: Browser, name: string) {
  for (const item of await b.$$('[data-testid="workspace-item"]')) {
    if ((await item.getText()).trim() === name) return item;
  }
  return null;
}

// Create a workspace via the new-workspace control (its name comes from window.prompt, stubbed) and
// return its id. The new workspace becomes the active one.
async function createWorkspace(b: Browser, name: string): Promise<string> {
  await stubPrompt(b, name);
  const add = await b.$('[data-testid="new-workspace"]');
  await add.waitForExist({ timeout: 10_000 });
  await add.click();
  let id = "";
  await b.waitUntil(
    async () => {
      const item = await workspaceItem(b, name);
      if (!item) return false;
      id = (await item.getAttribute("data-workspace-id")) ?? "";
      return id.length > 0;
    },
    { timeout: 10_000, timeoutMsg: `created workspace "${name}" did not appear in the switcher` },
  );
  return id;
}

async function selectWorkspace(b: Browser, name: string): Promise<void> {
  const item = await workspaceItem(b, name);
  if (!item) throw new Error(`workspace "${name}" not in switcher`);
  await item.click();
}

test("the switcher lists the Default workspace", async () => {
  const b = getApp();
  await resetToHome(b);
  expect(await workspaceItem(b, "Default")).not.toBeNull();
}, 120_000);

test("creating a workspace and selecting it re-scopes the sidebar in place", async () => {
  const b = getApp();
  await resetToHome(b);

  const wsName = uniqueName("WS");
  await createWorkspace(b, wsName); // becomes the active workspace

  // A project created while the new workspace is active belongs to it and shows in its sidebar.
  const projName = uniqueName("WsProj");
  const url = await createProject(b, projName);
  expect(url).toContain("/session/");
  await b.waitUntil(async () => (await b.$("body").getText()).includes(projName), {
    timeout: 10_000,
    timeoutMsg: "new project did not appear in its workspace's sidebar",
  });

  // Switching to Default re-scopes the sidebar away from that project — and opens no new window.
  await selectWorkspace(b, "Default");
  await b.waitUntil(async () => !(await b.$("body").getText()).includes(projName), {
    timeout: 10_000,
    timeoutMsg: "switching to Default did not drop the other workspace's project",
  });
  expect(await b.getUrl()).toBe(url);

  // Switching back brings it into view again.
  await selectWorkspace(b, wsName);
  await b.waitUntil(async () => (await b.$("body").getText()).includes(projName), {
    timeout: 10_000,
    timeoutMsg: "switching back did not restore the workspace's project",
  });
}, 120_000);

test("detaching a workspace surfaces its detached affordance", async () => {
  const b = getApp();
  await resetToHome(b);

  const wsName = uniqueName("WSD");
  const id = await createWorkspace(b, wsName);

  const detach = await b.$(`[data-testid="workspace-detach"][data-workspace-id="${id}"]`);
  await detach.waitForExist({ timeout: 10_000 });
  await detach.click();

  const indicator = await b.$(
    `[data-testid="workspace-detached-indicator"][data-workspace-id="${id}"]`,
  );
  await indicator.waitForExist({ timeout: 10_000 });
  expect(await indicator.isExisting()).toBe(true);
}, 120_000);
