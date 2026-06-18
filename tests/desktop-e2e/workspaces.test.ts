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

test("two workspaces keep their projects isolated in the sidebar", async () => {
  const b = getApp();
  await resetToHome(b);

  const wsA = uniqueName("WsA");
  await createWorkspace(b, wsA); // active = wsA
  const projA = uniqueName("ProjA");
  await createProject(b, projA); // belongs to wsA

  const wsB = uniqueName("WsB");
  await createWorkspace(b, wsB); // active = wsB
  const projB = uniqueName("ProjB");
  await createProject(b, projB); // belongs to wsB

  // wsA shows only its own project.
  await selectWorkspace(b, wsA);
  await b.waitUntil(
    async () => {
      const text = await b.$("body").getText();
      return text.includes(projA) && !text.includes(projB);
    },
    { timeout: 10_000, timeoutMsg: "wsA sidebar did not isolate to its own project" },
  );

  // wsB shows only its own project.
  await selectWorkspace(b, wsB);
  await b.waitUntil(
    async () => {
      const text = await b.$("body").getText();
      return text.includes(projB) && !text.includes(projA);
    },
    { timeout: 10_000, timeoutMsg: "wsB sidebar did not isolate to its own project" },
  );
}, 120_000);

test("re-opening a detached workspace focuses rather than opening a second window", async () => {
  const b = getApp();
  await resetToHome(b);

  const wsName = uniqueName("WSF");
  const id = await createWorkspace(b, wsName);

  const detach = await b.$(`[data-testid="workspace-detach"][data-workspace-id="${id}"]`);
  await detach.waitForExist({ timeout: 10_000 });
  await detach.click();

  // Once detached, the detach control is replaced by a focus control; clicking it focuses the
  // existing window (no second window) — the row stays detached, the detach control stays gone.
  const indicator = await b.$(
    `[data-testid="workspace-detached-indicator"][data-workspace-id="${id}"]`,
  );
  await indicator.waitForExist({ timeout: 10_000 });
  await indicator.click();

  expect(await indicator.isExisting()).toBe(true);
  const detachAgain = await b.$(`[data-testid="workspace-detach"][data-workspace-id="${id}"]`);
  expect(await detachAgain.isExisting()).toBe(false);
}, 120_000);

test("the switcher lists multiple workspaces alongside Default", async () => {
  const b = getApp();
  await resetToHome(b);

  const wsA = uniqueName("WsList-A");
  const wsB = uniqueName("WsList-B");
  await createWorkspace(b, wsA);
  await createWorkspace(b, wsB);

  expect(await workspaceItem(b, "Default")).not.toBeNull();
  expect(await workspaceItem(b, wsA)).not.toBeNull();
  expect(await workspaceItem(b, wsB)).not.toBeNull();
}, 120_000);

test("creating a workspace adds it to the switcher", async () => {
  const b = getApp();
  await resetToHome(b);

  const name = uniqueName("WsCreate");
  expect(await workspaceItem(b, name)).toBeNull(); // absent before
  await createWorkspace(b, name);
  expect(await workspaceItem(b, name)).not.toBeNull(); // present after
}, 120_000);

test("a newly created workspace is ordered last in the switcher", async () => {
  const b = getApp();
  await resetToHome(b);

  const first = uniqueName("WsOrd-A");
  const second = uniqueName("WsOrd-B");
  await createWorkspace(b, first);
  await createWorkspace(b, second);

  const names: string[] = [];
  for (const item of await b.$$('[data-testid="workspace-item"]')) {
    names.push((await item.getText()).trim());
  }
  // Default seeds at sort_order 0; each new workspace appends after the last.
  expect(names.indexOf("Default")).toBeLessThan(names.indexOf(first));
  expect(names.indexOf(first)).toBeLessThan(names.indexOf(second));
}, 120_000);
