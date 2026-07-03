import { expect, test } from "bun:test";

import { createProject, openTerminal, type Browser } from "./helpers";
import { getApp } from "./shared-app";

// Changing the terminal color scheme in the settings panel must re-theme an already-mounted
// terminal live (no respawn). The terminal pane's background tracks the scheme, so it is the
// observable proxy: github-dark = rgb(13, 17, 23), github-light = rgb(255, 255, 255). This guards
// the cross-component reactive path (panel write -> shared state -> mounted terminal re-render),
// which a per-component setting read silently broke.

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
  const b = getApp();

  await createProject(b, "scheme-e2e");
  await openTerminal(b);

  await (await b.$('[aria-label="Settings"]')).click();
  const schemeSelect = await b.$('select[aria-label="Terminal scheme"]');
  await schemeSelect.waitForExist({ timeout: 10_000 });

  // Establish a known scheme on the mounted terminal (state-independent), then flip it; the
  // background flips live with no respawn. dark = rgb(13, 17, 23), light = rgb(255, 255, 255).
  await selectValue(b, "Terminal scheme", "github-dark");
  await b.waitUntil(async () => (await terminalBackground(b)) === "rgb(13, 17, 23)", {
    timeout: 15_000,
    timeoutMsg: "setting the dark scheme did not reach the mounted terminal",
  });

  await selectValue(b, "Terminal scheme", "github-light");
  await b.waitUntil(async () => (await terminalBackground(b)) === "rgb(255, 255, 255)", {
    timeout: 10_000,
    timeoutMsg: "terminal scheme change did not reach the mounted terminal",
  });
  expect(await terminalBackground(b)).toBe("rgb(255, 255, 255)");
}, 120_000);
