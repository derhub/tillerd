import { expect, test } from "bun:test";

import { createProject, observePause, resetToHome, uniqueName, type Browser } from "./helpers";
import { getApp } from "./shared-app";

// Native windows are unreachable under WebDriver, so detach is asserted via its DOM affordance.

async function workspaceItem(b: Browser, name: string) {
  for (const item of await b.$$('[data-testid="workspace-item"]')) {
    if ((await item.getText()).trim() === name) return item;
  }
  return null;
}

// New workspace creates a placeholder and opens an inline rename input (no native prompt); type the
// name and confirm. Returns the new workspace's id; the created workspace becomes active.
async function createWorkspace(b: Browser, name: string): Promise<string> {
  const add = await b.$('[data-testid="new-workspace"]');
  await add.waitForExist({ timeout: 10_000 });
  await add.click();
  const input = await b.$('[data-testid="inline-rename-input"]');
  await input.waitForExist({ timeout: 10_000 });
  await input.setValue(name);
  await b.keys(["Enter"]);
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
  await createWorkspace(b, wsName);

  const projName = uniqueName("WsProj");
  const url = await createProject(b, projName);
  expect(url).toContain("/session/");
  await b.waitUntil(async () => (await b.$("body").getText()).includes(projName), {
    timeout: 10_000,
    timeoutMsg: "new project did not appear in its workspace's sidebar",
  });

  await selectWorkspace(b, "Default");
  await b.waitUntil(async () => !(await b.$("body").getText()).includes(projName), {
    timeout: 10_000,
    timeoutMsg: "switching to Default did not drop the other workspace's project",
  });
  expect(await b.getUrl()).toBe(url);

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
  // Populate the workspace before detaching -- a real detach carries a project and its session,
  // not an empty shell. createProject creates (and routes to) a default session in this workspace.
  await createProject(b, uniqueName("WSDProj"));

  const detach = await b.$(`[data-testid="workspace-detach"][data-workspace-id="${id}"]`);
  await detach.waitForExist({ timeout: 10_000 });
  await detach.click();

  const indicator = await b.$(
    `[data-testid="workspace-detached-indicator"][data-workspace-id="${id}"]`,
  );
  await indicator.waitForExist({ timeout: 10_000 });
  expect(await indicator.isExisting()).toBe(true);

  // Re-attach so the detached child window does not linger into later specs (a stale child
  // window keeps its own service-health/orchestrator view and clutters the shared run).
  await observePause(b);
  await indicator.click();
  await (
    await b.$(`[data-testid="workspace-detach"][data-workspace-id="${id}"]`)
  ).waitForExist({ timeout: 10_000 });
}, 120_000);

test("two workspaces keep their projects isolated in the sidebar", async () => {
  const b = getApp();
  await resetToHome(b);

  const wsA = uniqueName("WsA");
  await createWorkspace(b, wsA);
  const projA = uniqueName("ProjA");
  await createProject(b, projA);

  const wsB = uniqueName("WsB");
  await createWorkspace(b, wsB);
  const projB = uniqueName("ProjB");
  await createProject(b, projB);

  await selectWorkspace(b, wsA);
  await b.waitUntil(
    async () => {
      const text = await b.$("body").getText();
      return text.includes(projA) && !text.includes(projB);
    },
    { timeout: 10_000, timeoutMsg: "wsA sidebar did not isolate to its own project" },
  );

  await selectWorkspace(b, wsB);
  await b.waitUntil(
    async () => {
      const text = await b.$("body").getText();
      return text.includes(projB) && !text.includes(projA);
    },
    { timeout: 10_000, timeoutMsg: "wsB sidebar did not isolate to its own project" },
  );
}, 120_000);

test("two workspaces keep their session controls isolated in the sidebar", async () => {
  const b = getApp();
  await resetToHome(b);

  // Each workspace gets a project; createProject also makes a default session within it. The
  // per-project "New session in <project>" control is the session-scope affordance and is
  // rendered for every visible project, so its presence tracks the active workspace.
  const wsA = uniqueName("WsA");
  await createWorkspace(b, wsA);
  const projA = uniqueName("ProjA");
  await createProject(b, projA);

  const wsB = uniqueName("WsB");
  await createWorkspace(b, wsB);
  const projB = uniqueName("ProjB");
  await createProject(b, projB);

  const sessionControl = (proj: string) =>
    b.$(`button[title="New session in ${proj}"]`).then((el) => el.isExisting());

  // wsB is current (its project was just created): only projB's session control is present.
  await (await b.$(`button[title="New session in ${projB}"]`)).waitForExist({ timeout: 10_000 });
  expect(await sessionControl(projA)).toBe(false);

  // Selecting wsA re-scopes the sidebar to its own session control and drops wsB's.
  await selectWorkspace(b, wsA);
  await b.waitUntil(async () => (await sessionControl(projA)) && !(await sessionControl(projB)), {
    timeout: 10_000,
    timeoutMsg: "wsA sidebar did not isolate to its own session control",
  });
}, 120_000);

test("re-attaching a detached workspace from the parent restores its detach control", async () => {
  const b = getApp();
  await resetToHome(b);

  const wsName = uniqueName("WSF");
  const id = await createWorkspace(b, wsName);
  // Detach (then re-attach) a populated workspace, not an empty shell.
  await createProject(b, uniqueName("WSFProj"));

  const detach = await b.$(`[data-testid="workspace-detach"][data-workspace-id="${id}"]`);
  await detach.waitForExist({ timeout: 10_000 });
  await detach.click();

  const indicator = await b.$(
    `[data-testid="workspace-detached-indicator"][data-workspace-id="${id}"]`,
  );
  await indicator.waitForExist({ timeout: 10_000 });
  // Clicking the indicator closes the child window from the parent, which re-attaches the
  // workspace -- the row returns to its detach control.
  await observePause(b);
  await indicator.click();

  const detachAgain = await b.$(`[data-testid="workspace-detach"][data-workspace-id="${id}"]`);
  await detachAgain.waitForExist({ timeout: 10_000 });
  expect(
    await b
      .$(`[data-testid="workspace-detached-indicator"][data-workspace-id="${id}"]`)
      .then((el) => el.isExisting()),
  ).toBe(false);
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
  expect(await workspaceItem(b, name)).toBeNull();
  await createWorkspace(b, name);
  expect(await workspaceItem(b, name)).not.toBeNull();
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
  expect(names.indexOf("Default")).toBeLessThan(names.indexOf(first));
  expect(names.indexOf(first)).toBeLessThan(names.indexOf(second));
}, 120_000);

// A child window is the same app at a `?w=...` query the shell parses on mount. WebDriver cannot
// reach a real child window, so navigate the MAIN webview to the ROOT query form (`/?w=...`) to
// reproduce its scoped sidebar. Use a root-relative URL: a non-root path 404s on the custom scheme,
// and `new URL(tauriUrl).origin` is `null` (the tauri scheme has no standard origin).
async function openWindowUrl(b: Browser, query: string): Promise<void> {
  await b.execute((q: string) => window.location.assign(q), query);
}
async function restoreMainWindow(b: Browser): Promise<void> {
  await b.execute(() => window.location.assign("/"));
  await b.waitUntil(async () => (await b.$("body").getText()).includes("services: ready"), {
    timeout: 45_000,
    timeoutMsg: "app did not return to the main window",
  });
}

test("a workspace window scopes the sidebar to its own projects", async () => {
  const b = getApp();
  await resetToHome(b);
  const idA = await createWorkspace(b, uniqueName("WWA"));
  const projA = uniqueName("WWAProj");
  await createProject(b, projA);
  await createWorkspace(b, uniqueName("WWB"));
  const projB = uniqueName("WWBProj");
  await createProject(b, projB);

  await openWindowUrl(b, `/?w=workspace&workspace=${idA}`);
  await b.waitUntil(
    async () => {
      const t = await b.$("body").getText();
      return t.includes(projA) && !t.includes(projB);
    },
    { timeout: 30_000, timeoutMsg: "workspace window did not scope to its own projects" },
  );
  await restoreMainWindow(b);
}, 180_000);

test("a project window scopes the sidebar to that project only", async () => {
  const b = getApp();
  await resetToHome(b);
  await createWorkspace(b, uniqueName("PWA"));
  const projA = uniqueName("PWAProj");
  await createProject(b, projA);
  const projB = uniqueName("PWBProj");
  await createProject(b, projB);

  // The project window query needs the project id; read it from the sidebar row (the project is in
  // the active workspace right after createProject).
  const projAId = await b.execute((name: string) => {
    const els = Array.from(document.querySelectorAll('[data-testid="project-name"]'));
    return els.find((e) => e.textContent?.includes(name))?.getAttribute("data-project-id") ?? null;
  }, projA);
  expect(typeof projAId).toBe("string");

  await openWindowUrl(b, `/?w=project&project=${projAId as string}`);
  await b.waitUntil(
    async () => {
      const t = await b.$("body").getText();
      return t.includes(projA) && !t.includes(projB);
    },
    { timeout: 30_000, timeoutMsg: "project window did not scope to its own project" },
  );
  await restoreMainWindow(b);
}, 180_000);
