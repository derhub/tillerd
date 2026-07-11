import { expect, test } from "bun:test";

import { createProject, openView, uniqueName } from "./helpers";
import { getApp } from "./shared-app";

// Terse smoke coverage for three managers introduced across the ui-settings-editor and
// ui-panel-compound work: the settings editor's section nav, the Commands library's create flow,
// and the Templates view's two sections. Not exhaustive -- each manager's own CRUD edge cases are
// unit-tested; this only proves each one actually renders and takes one real action end to end.

test("settings editor, Commands create, and Templates sections all render and work", async () => {
  const b = getApp();
  // Templates' "This project" section only renders with an active project (useActiveProject),
  // so seed one before visiting it.
  await createProject(b, uniqueName("SettingsSmoke"));

  // -- Settings editor: 7 sections (General, Appearance, Terminal, Keybindings, Profiles,
  // Themes, and Project -- shown because a project is active), Keybindings shows its preset
  // select --
  await (await b.$('[aria-label="Settings"]')).click();
  const editor = await b.$('[data-testid="settings-editor"]');
  await editor.waitForExist({ timeout: 10_000 });
  const sectionButtons = await b.$$('nav[aria-label="Settings sections"] button');
  expect(sectionButtons.length).toBe(7);

  await (await b.$('[data-testid="settings-section-keybindings"]')).click();
  await (await b.$('[aria-label="Keybinding preset"]')).waitForExist({ timeout: 10_000 });

  // -- Commands view: create a custom command, see it listed --
  await openView(b, "Commands");
  const commandName = uniqueName("Smoke Command");
  await (await b.$('[data-testid="command-create-button"]')).click();
  const dialog = await b.$('[data-testid="command-form-dialog"]');
  await dialog.waitForExist({ timeout: 10_000 });
  await (await b.$('[data-testid="command-form-name"]')).setValue(commandName);
  await (await b.$('[data-testid="command-form-cli"]')).setValue("echo");
  await (await b.$('[data-testid="command-form-save"]')).click();
  await dialog.waitForExist({ timeout: 10_000, reverse: true });

  await b.waitUntil(
    async () => {
      for (const row of await b.$$('[data-testid="command-name"]')) {
        if ((await row.getText()) === commandName) return true;
      }
      return false;
    },
    { timeout: 10_000, timeoutMsg: "created command did not appear in the Commands list" },
  );

  // -- Templates view: both sections render (Library always; "This project" with one active) --
  await openView(b, "Templates");
  await (await b.$("h3*=Library")).waitForExist({ timeout: 10_000 });
  await (
    await b.$("h3*=This project")
  ).waitForExist({
    timeout: 10_000,
    timeoutMsg: "the project-scoped Templates section did not render with an active project",
  });

  // The active sidebar view is a global setting shared by the rest of this run's suite (this test
  // shares the app, not its own launch); leaving it on Templates hides the "New project" button
  // that every other spec's `resetToHome` baseline depends on, wedging every test after this one.
  // Restore the default view this test switched away from.
  await openView(b, "Sessions");
}, 120_000);
