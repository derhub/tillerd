import { afterEach, expect, test } from "bun:test";
import { type Browser, createProject, launchReadyApp } from "./helpers";

let browser: Browser | undefined;
afterEach(async () => {
  await browser?.deleteSession();
  browser = undefined;
});

// ── Inline Rename ──────────────────────────────────────────────────────────

test("rename project by double-clicking and pressing Enter", async () => {
  const b = (browser = await launchReadyApp());
  const projectName = `Project ${Date.now()}`;
  const newName = `Renamed ${Date.now()}`;

  await createProject(b, projectName);

  // Find the project header in the sidebar (uppercase, appears as a section header)
  const projectHeader = await b.$(`span*=${projectName}`);
  await projectHeader.waitForExist({ timeout: 10_000 });

  // Double-click to activate inline edit
  await projectHeader.doubleClick();

  // Input should now be focused and have the existing name selected
  const input = await b.$("input");
  await input.waitForExist({ timeout: 5_000 });
  expect(await input.getValue()).toBe(projectName);

  // Clear and type new name
  await input.clearValue();
  await input.setValue(newName);

  // Press Enter to confirm
  await input.keys(["Return"]);

  // Input should close, new name should appear
  await b.waitUntil(
    async () => {
      const text = await b.$("body").getText();
      return text.includes(newName) && !text.includes(projectName);
    },
    { timeout: 10_000, timeoutMsg: "project rename did not persist in sidebar" },
  );
}, 120_000);

test("cancel project rename by pressing Escape", async () => {
  const b = (browser = await launchReadyApp());
  const projectName = `Project ${Date.now()}`;

  await createProject(b, projectName);

  // Double-click to activate inline edit
  const projectHeader = await b.$(`span*=${projectName}`);
  await projectHeader.doubleClick();

  const input = await b.$("input");
  await input.waitForExist({ timeout: 5_000 });

  // Modify and press Escape
  await input.clearValue();
  await input.setValue("New Name");
  await input.keys(["Escape"]);

  // Input should close, original name should remain
  await b.waitUntil(
    async () => (await b.$("body").getText()).includes(projectName),
    { timeout: 10_000, timeoutMsg: "project name reverted after Escape" },
  );
}, 120_000);

// ── Context Menu ───────────────────────────────────────────────────────────

test("right-click project opens context menu", async () => {
  const b = (browser = await launchReadyApp());
  const projectName = `Project ${Date.now()}`;

  await createProject(b, projectName);

  // Right-click on project header
  const projectHeader = await b.$(`span*=${projectName}`);
  await projectHeader.waitForExist({ timeout: 10_000 });
  await projectHeader.click({ button: 2 }); // button: 2 = right-click

  // Context menu should appear with actions
  const menu = await b.$("[role=menu]");
  await menu.waitForExist({ timeout: 5_000 });

  const menuText = await menu.getText();
  expect(menuText).toMatch(/rename/i);
  expect(menuText).toMatch(/delete/i);
  expect(menuText).toMatch(/archive/i);
}, 120_000);

// ── Delete ─────────────────────────────────────────────────────────────────

test("delete project after confirming dialog", async () => {
  const b = (browser = await launchReadyApp());
  const projectName = `Project ${Date.now()}`;

  await createProject(b, projectName);

  // Right-click and select Delete
  const projectHeader = await b.$(`span*=${projectName}`);
  await projectHeader.click({ button: 2 });

  const menu = await b.$("[role=menu]");
  await menu.waitForExist({ timeout: 5_000 });

  const deleteBtn = await menu.$("button*=Delete");
  await deleteBtn.click();

  // Confirmation dialog should appear
  const dialog = await b.$("[role=alertdialog]");
  await dialog.waitForExist({ timeout: 5_000 });

  // Confirm delete
  const confirmBtn = await dialog.$("button*=Delete");
  await confirmBtn.click();

  // Project should disappear from sidebar
  await b.waitUntil(
    async () => {
      const text = await b.$("body").getText();
      return !text.includes(projectName);
    },
    { timeout: 10_000, timeoutMsg: "deleted project still appears in sidebar" },
  );
}, 120_000);
