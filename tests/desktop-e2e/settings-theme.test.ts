import { expect, test } from "bun:test";
import { type Browser } from "./helpers";
import { getApp } from "./shared-app";

// The settings gear sits in the shell's bottom-right cluster. Clicking it opens a non-modal
// popover with a Theme select. Choosing an appearance applies it in real time by toggling the
// `.dark` class on the document root -- a real-webview concern (happy-dom has no real paint and
// the popover renders in a portal), so it lives here; the apply/persist logic is unit-tested.

// Set a React controlled <select> by value and fire `change` -- webdriverio's selectBy* helpers
// do not reliably trigger React's onChange under WKWebView.
async function selectValue(b: Browser, ariaLabel: string, value: string): Promise<void> {
  await b.execute(
    (label: string, v: string) => {
      const el = document.querySelector<HTMLSelectElement>(`select[aria-label="${label}"]`);
      if (!el) throw new Error(`select not found: ${label}`);
      el.value = v;
      el.dispatchEvent(new Event("change", { bubbles: true }));
    },
    ariaLabel,
    value,
  );
}

function hasDarkClass(b: Browser): Promise<boolean> {
  return b.execute(() => document.documentElement.classList.contains("dark"));
}

test("selecting a theme in the settings panel toggles the appearance in real time", async () => {
  const b = getApp();

  // Open the settings popover from the bottom-right cluster.
  await (await b.$('[aria-label="Settings"]')).click();
  const panel = await b.$('[data-slot="popover-content"]');
  await panel.waitForExist({ timeout: 10_000 });
  await (await b.$('select[aria-label="Theme"]')).waitForExist({ timeout: 10_000 });

  // Cycle light -> dark -> light. Each selection applies live (toggles `.dark` on the root with
  // no reload), in both directions, repeatedly. State-independent: the test sets each appearance
  // rather than assuming the persisted starting one.
  const steps: ReadonlyArray<readonly ["light" | "dark", boolean]> = [
    ["light", false],
    ["dark", true],
    ["light", false],
  ];
  for (const [value, wantDark] of steps) {
    await selectValue(b, "Theme", value);
    await b.waitUntil(async () => (await hasDarkClass(b)) === wantDark, {
      timeout: 10_000,
      timeoutMsg: `selecting ${value} did not ${wantDark ? "add" : "remove"} the dark class`,
    });
  }
  expect(await hasDarkClass(b)).toBe(false);
}, 120_000);
