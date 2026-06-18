import { expect, test } from "bun:test";
import { createProject, openTerminal, uniqueName } from "./helpers";
import { getApp } from "./shared-app";

// Panel detach / multi-window (roadmap 0.0.11), parent-webview scenarios. tauri-webdriver drives a
// single webview and cannot invoke commands or emit events from `execute`, so child-window
// existence, focus-raise, and re-attach are not observable here — those are covered by the desktop
// command-contract test, the renderer unit tests, and manual verification. What IS observable is the
// parent's DOM reaction to each action, asserted below.

// Scenario (multi-window 1.2): an empty panel exposes no detach affordance.
test("an empty panel shows no detach affordance", async () => {
  const b = getApp();
  await createProject(b, uniqueName("Detach"));

  // A fresh session opens to an empty leaf; nothing is spawned, so there is no live surface.
  const detach = await b.$('button[aria-label="Detach"]');
  expect(await detach.isExisting()).toBe(false);
}, 120_000);

// Scenario (multi-window 1.1, parent side): detaching a live terminal replaces the panel with a
// greyed placeholder bearing a Focus button, and the live surface leaves the parent.
test("detaching a terminal replaces it with a Focus placeholder", async () => {
  const b = getApp();
  await createProject(b, uniqueName("Detach"));
  const surface = await openTerminal(b);
  expect(surface).toBeTruthy();

  const detach = await b.$('button[aria-label="Detach"]');
  await detach.waitForExist({ timeout: 10_000 });
  await detach.click();

  const placeholder = await b.$('[data-testid="detached-placeholder"]');
  await placeholder.waitForExist({ timeout: 10_000 });
  const focus = await b.$('button[aria-label="Focus detached window"]');
  expect(await focus.isExisting()).toBe(true);

  // The live terminal surface no longer renders in the parent window.
  const surfaceEl = await b.$("[data-surface-id]");
  expect(await surfaceEl.isExisting()).toBe(false);
}, 120_000);

// Scenario (open project in new window, parent side): the context-menu action marks the parent
// project row with a pending-detach indicator.
test("opening a project in a new window marks the parent row", async () => {
  const b = getApp();
  const name = uniqueName("Detach");
  await createProject(b, name);

  // Dispatch the context menu on the project heading (native right-click is unreliable under
  // WebDriver), then click "Open in new window".
  await b.execute((projectName: string) => {
    const heading = Array.from(document.querySelectorAll("span")).find(
      (el) => el.textContent === projectName,
    );
    heading?.dispatchEvent(
      new MouseEvent("contextmenu", { bubbles: true, clientX: 40, clientY: 40 }),
    );
  }, name);

  // The 0.0.12 project menu carries the full action list (Rename / Open in new window / Delete), so
  // target the open action by its label rather than menu position.
  const openItem = await b.$("button*=Open in new window");
  await openItem.waitForExist({ timeout: 10_000 });
  await openItem.click();

  const indicator = await b.$('[data-testid="project-detached-indicator"]');
  await indicator.waitForExist({ timeout: 10_000 });
  expect(await indicator.isExisting()).toBe(true);
}, 120_000);
