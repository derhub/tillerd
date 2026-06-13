import { afterEach, expect, test } from "bun:test";
import { type Browser, createProject, launchReadyApp, openTerminal } from "./helpers";

// Changing the terminal color scheme in the settings panel must re-theme an already-mounted
// terminal live (no respawn). The terminal pane's background tracks the scheme, so it is the
// observable proxy: github-dark = rgb(13, 17, 23), github-light = rgb(255, 255, 255). This guards
// the cross-component reactive path (panel write -> shared state -> mounted terminal re-render),
// which a per-component setting read silently broke.

let browser: Browser | undefined;
afterEach(async () => {
  await browser?.deleteSession();
  browser = undefined;
});

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

function terminalBackground(b: Browser): Promise<string> {
  return b.execute(() => {
    const el = document.querySelector("[data-surface-id]");
    return el ? getComputedStyle(el).backgroundColor : "";
  });
}

test("changing the terminal scheme re-themes a mounted terminal live", async () => {
  const b = await launchReadyApp();
  browser = b;

  await createProject(b, "scheme-e2e");
  await openTerminal(b);

  // Default scheme is github-dark.
  await b.waitUntil(async () => (await terminalBackground(b)) === "rgb(13, 17, 23)", {
    timeout: 15_000,
    timeoutMsg: "terminal did not start with the dark scheme background",
  });

  // Open the settings popover and switch the terminal scheme to light.
  await (await b.$('[aria-label="Settings"]')).click();
  const schemeSelect = await b.$('select[aria-label="Terminal scheme"]');
  await schemeSelect.waitForExist({ timeout: 10_000 });
  await selectValue(b, "Terminal scheme", "github-light");

  // The mounted terminal's background flips live, without a respawn.
  await b.waitUntil(async () => (await terminalBackground(b)) === "rgb(255, 255, 255)", {
    timeout: 10_000,
    timeoutMsg: "terminal scheme change did not reach the mounted terminal",
  });
  expect(await terminalBackground(b)).toBe("rgb(255, 255, 255)");
}, 120_000);
