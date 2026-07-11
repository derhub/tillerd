import { expect, test } from "bun:test";

import { type Browser } from "./helpers";
import { getApp } from "./shared-app";

// The settings gear sits in the status bar's bottom-right cluster and now navigates to the
// /settings editor route (retired popover -- see ui-settings-editor spec). Appearance is the
// default section, so the Theme control is visible immediately. Choosing an appearance applies
// it in real time by toggling the `.dark` class on the document root -- a real-webview concern
// (happy-dom has no real paint), so it lives here; the apply/persist logic is unit-tested.

// The Theme control is a shadcn/base-ui Select (a button trigger + a portaled listbox), not a
// native <select> -- open it and click the option by its visible text via a genuine WebDriver
// click (a synthetic DOM click on the option does not reliably register with the popup's pointer
// interaction manager under WKWebView, mirroring the popover-dismiss note in command-center.test.ts).
async function selectOption(
  b: Browser,
  triggerAriaLabel: string,
  optionText: string,
): Promise<void> {
  await (await b.$(`[aria-label="${triggerAriaLabel}"]`)).click();
  const option = await b.$(`//*[@data-slot="select-item" and contains(., "${optionText}")]`);
  await option.waitForExist({ timeout: 10_000 });
  await option.click();
}

function hasDarkClass(b: Browser): Promise<boolean> {
  return b.execute(() => document.documentElement.classList.contains("dark"));
}

test("selecting a theme in the settings editor toggles the appearance in real time", async () => {
  const b = getApp();

  // Navigate to the settings editor from the status bar gear.
  await (await b.$('[aria-label="Settings"]')).click();
  const editor = await b.$('[data-testid="settings-editor"]');
  await editor.waitForExist({ timeout: 10_000 });
  await (await b.$('[aria-label="Theme"]')).waitForExist({ timeout: 10_000 });

  // Cycle light -> dark -> light -> dark. Each selection applies live (toggles `.dark` on the root
  // with no reload), in both directions, repeatedly. State-independent: the test sets each
  // appearance rather than assuming the persisted starting one, and ends on dark so the shared app
  // is left at the product default (dark) for later tests and screenshots.
  const steps: ReadonlyArray<readonly ["light" | "dark", boolean]> = [
    ["light", false],
    ["dark", true],
    ["light", false],
    ["dark", true],
  ];
  for (const [value, wantDark] of steps) {
    await selectOption(b, "Theme", value);
    await b.waitUntil(async () => (await hasDarkClass(b)) === wantDark, {
      timeout: 10_000,
      timeoutMsg: `selecting ${value} did not ${wantDark ? "add" : "remove"} the dark class`,
    });
  }
  expect(await hasDarkClass(b)).toBe(true);
}, 120_000);
