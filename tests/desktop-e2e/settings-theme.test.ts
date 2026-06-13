import { afterEach, expect, test } from "bun:test";
import { type Browser, launchReadyApp } from "./helpers";

// The settings gear sits in the shell's bottom-right cluster. Clicking it opens a non-modal
// popover with a Theme select. Choosing an appearance applies it in real time by toggling the
// `.dark` class on the document root — a real-webview concern (happy-dom has no real paint and
// the popover renders in a portal), so it lives here; the apply/persist logic is unit-tested.

let browser: Browser | undefined;
afterEach(async () => {
  await browser?.deleteSession();
  browser = undefined;
});

// Set a React controlled <select> by value and fire `change` — webdriverio's selectBy* helpers
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
  const b = await launchReadyApp();
  browser = b;

  // Default appearance is dark.
  expect(await hasDarkClass(b)).toBe(true);

  // Open the settings popover from the bottom-right cluster.
  await (await b.$('[aria-label="Settings"]')).click();
  const panel = await b.$('[data-slot="popover-content"]');
  await panel.waitForExist({ timeout: 10_000 });
  const themeSelect = await b.$('select[aria-label="Theme"]');
  await themeSelect.waitForExist({ timeout: 10_000 });

  // Switch to light: the root drops `.dark` immediately, no reload.
  await selectValue(b, "Theme", "light");
  await b.waitUntil(async () => !(await hasDarkClass(b)), {
    timeout: 10_000,
    timeoutMsg: "selecting light did not remove the dark class",
  });

  // Switch back to dark: the change applies live the other direction too.
  await selectValue(b, "Theme", "dark");
  await b.waitUntil(async () => await hasDarkClass(b), {
    timeout: 10_000,
    timeoutMsg: "selecting dark did not restore the dark class",
  });
}, 120_000);
