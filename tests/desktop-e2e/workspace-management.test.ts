import { expect, test } from "bun:test";
import { createProject, type Browser, uniqueName } from "./helpers";
import { getApp } from "./shared-app";

// WebdriverIO's synthetic click/doubleClick/right-click do not reliably reach React's delegated
// event listeners in WKWebView (see the `testing` memory). Dispatch real DOM MouseEvents on the
// target element instead — they bubble to React's root listener and fire the JSX handlers. Keyboard
// input on a focused <input> uses the real WebDriver key path, which does fire onChange/onKeyDown.

async function dispatchMouse(b: Browser, selector: string, type: "dblclick" | "contextmenu") {
  await b.execute(
    (sel, evType) => {
      const el = document.querySelector(sel);
      if (!el) return;
      const r = el.getBoundingClientRect();
      el.dispatchEvent(
        new MouseEvent(evType, {
          bubbles: true,
          cancelable: true,
          clientX: r.left + r.width / 2,
          clientY: r.top + r.height / 2,
          button: evType === "contextmenu" ? 2 : 0,
        }),
      );
    },
    selector,
    type,
  );
}

// The desktop e2e suite shares one TILLERD_DIR, so many projects accumulate and list order is not
// positional. Target a project by its unique name, not the first matching header.
async function dispatchProjectMouse(b: Browser, name: string, type: "dblclick" | "contextmenu") {
  await b.execute(
    (nm, evType) => {
      const el = Array.from(document.querySelectorAll('[data-testid="project-name"]')).find(
        (e) => e.textContent === nm,
      );
      if (!el) return;
      const r = el.getBoundingClientRect();
      el.dispatchEvent(
        new MouseEvent(evType, {
          bubbles: true,
          cancelable: true,
          clientX: r.left + r.width / 2,
          clientY: r.top + r.height / 2,
          button: evType === "contextmenu" ? 2 : 0,
        }),
      );
    },
    name,
    type,
  );
}

// Wait until this test's uniquely-named project header is present in the sidebar.
async function waitForProject(b: Browser, name: string) {
  await b.waitUntil(
    async () => {
      for (const h of await b.$$('[data-testid="project-name"]')) {
        if ((await h.getText()) === name) return true;
      }
      return false;
    },
    { timeout: 10_000, timeoutMsg: `project header "${name}" did not appear` },
  );
}

// The context menu closes on a `mousedown` outside it (ContextMenuShell listens on window).
async function mousedownBody(b: Browser) {
  await b.execute(() => {
    document.body.dispatchEvent(new MouseEvent("mousedown", { bubbles: true }));
  });
}

// ── Inline Rename ──────────────────────────────────────────────────────────

test("rename project by double-clicking and pressing Enter", async () => {
  const b = getApp();
  const projectName = uniqueName("Project");
  const newName = uniqueName("Renamed");

  await createProject(b, projectName);

  await waitForProject(b, projectName);
  await dispatchProjectMouse(b, projectName, "dblclick");

  const input = await b.$('[data-testid="inline-rename-input"]');
  await input.waitForExist({ timeout: 5_000 });

  await input.setValue(newName);
  await b.keys(["Return"]);

  await b.waitUntil(async () => (await b.$("body").getText()).includes(newName), {
    timeout: 10_000,
    timeoutMsg: "project rename did not persist in sidebar",
  });
}, 120_000);

test("cancel project rename by pressing Escape", async () => {
  const b = getApp();
  const projectName = uniqueName("Project");

  await createProject(b, projectName);

  await waitForProject(b, projectName);
  await dispatchProjectMouse(b, projectName, "dblclick");
  const input = await b.$('[data-testid="inline-rename-input"]');
  await input.waitForExist({ timeout: 5_000 });

  await input.setValue("New Name");
  await b.keys(["Escape"]);

  // Input closes, original name remains.
  await b.waitUntil(async () => (await b.$("body").getText()).includes(projectName), {
    timeout: 10_000,
    timeoutMsg: "project name reverted after Escape",
  });
}, 120_000);

// ── Context Menu ───────────────────────────────────────────────────────────

test("right-click project opens context menu", async () => {
  const b = getApp();
  const projectName = uniqueName("Project");

  await createProject(b, projectName);

  await waitForProject(b, projectName);
  await dispatchProjectMouse(b, projectName, "contextmenu");

  const menu = await b.$("[role=menu]");
  await menu.waitForExist({ timeout: 5_000 });

  const menuText = await menu.getText();
  expect(menuText).toMatch(/rename/i);
  expect(menuText).toMatch(/delete/i);
}, 120_000);

// ── Delete ─────────────────────────────────────────────────────────────────

test("delete project after confirming dialog", async () => {
  const b = getApp();
  const projectName = uniqueName("Project");

  await createProject(b, projectName);

  await waitForProject(b, projectName);
  await dispatchProjectMouse(b, projectName, "contextmenu");

  const menu = await b.$("[role=menu]");
  await menu.waitForExist({ timeout: 5_000 });
  await (await menu.$("button*=Delete")).click();

  const dialog = await b.$("[role=alertdialog]");
  await dialog.waitForExist({ timeout: 5_000 });
  await (await dialog.$("button*=Delete")).click();

  // The confirm handler dismisses the dialog only after the delete resolves; a stuck-open dialog
  // means the IPC rejected.
  await dialog.waitForExist({
    reverse: true,
    timeout: 10_000,
    timeoutMsg: "delete dialog stayed open",
  });

  // This test's uniquely-named project header disappears once the delete cascade completes (other
  // tests' projects remain — the suite shares one TILLERD_DIR).
  await b.waitUntil(
    async () => {
      for (const h of await b.$$('[data-testid="project-name"]')) {
        if ((await h.getText()) === projectName) return false;
      }
      return true;
    },
    { timeout: 10_000, timeoutMsg: "deleted project still appears in the sidebar" },
  );
}, 120_000);

test("cancel project deletion leaves the project in place", async () => {
  const b = getApp();
  const projectName = uniqueName("Project");

  await createProject(b, projectName);

  await waitForProject(b, projectName);
  await dispatchProjectMouse(b, projectName, "contextmenu");

  const menu = await b.$("[role=menu]");
  await menu.waitForExist({ timeout: 5_000 });
  await (await menu.$("button*=Delete")).click();

  const dialog = await b.$("[role=alertdialog]");
  await dialog.waitForExist({ timeout: 5_000 });
  await (await dialog.$("button*=Cancel")).click();

  await b.waitUntil(async () => (await b.$("body").getText()).includes(projectName), {
    timeout: 10_000,
    timeoutMsg: "project vanished after cancelling deletion",
  });
}, 120_000);

// ── Session inline rename / context menu ────────────────────────────────────

test("rename session by double-clicking and pressing Enter", async () => {
  const b = getApp();
  const projectName = uniqueName("Project");
  const newTitle = uniqueName("Renamed Session");

  const firstSessionUrl = await createProject(b, projectName);
  const sessionId = firstSessionUrl.split("/session/")[1];

  const rowSel = `a[href="/session/${sessionId}"]`;
  await (await b.$(rowSel)).waitForExist({ timeout: 10_000 });
  await dispatchMouse(b, rowSel, "dblclick");

  const input = await b.$('[data-testid="inline-rename-input"]');
  await input.waitForExist({ timeout: 5_000 });
  await input.setValue(newTitle);
  await b.keys(["Return"]);

  await b.waitUntil(async () => (await b.$("body").getText()).includes(newTitle), {
    timeout: 10_000,
    timeoutMsg: "session rename did not persist in sidebar",
  });
}, 120_000);

test("right-click session opens a context menu with rename, archive, delete", async () => {
  const b = getApp();
  const projectName = uniqueName("Project");

  const firstSessionUrl = await createProject(b, projectName);
  const sessionId = firstSessionUrl.split("/session/")[1];

  const rowSel = `a[href="/session/${sessionId}"]`;
  await (await b.$(rowSel)).waitForExist({ timeout: 10_000 });
  await dispatchMouse(b, rowSel, "contextmenu");

  const menu = await b.$("[role=menu]");
  await menu.waitForExist({ timeout: 5_000 });
  const menuText = await menu.getText();
  expect(menuText).toMatch(/rename/i);
  expect(menuText).toMatch(/archive/i);
  expect(menuText).toMatch(/delete/i);
}, 120_000);

test("session context menu closes on outside click", async () => {
  const b = getApp();
  const projectName = uniqueName("Project");

  const firstSessionUrl = await createProject(b, projectName);
  const sessionId = firstSessionUrl.split("/session/")[1];

  const rowSel = `a[href="/session/${sessionId}"]`;
  await (await b.$(rowSel)).waitForExist({ timeout: 10_000 });
  await dispatchMouse(b, rowSel, "contextmenu");

  const menu = await b.$("[role=menu]");
  await menu.waitForExist({ timeout: 5_000 });

  await mousedownBody(b);
  await b.waitUntil(async () => !(await menu.isExisting()), {
    timeout: 5_000,
    timeoutMsg: "context menu stayed open after an outside click",
  });
}, 120_000);
