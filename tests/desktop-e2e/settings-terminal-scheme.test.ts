import { expect, test } from "bun:test";

import { createProject, openTerminal, type Browser } from "./helpers";
import { getApp } from "./shared-app";

// Changing the terminal color scheme in the settings editor's Terminal section must re-theme an
// already-mounted terminal live (no respawn). The terminal pane's background tracks the scheme,
// so it is the observable proxy: github-dark = rgb(13, 17, 23), github-light = rgb(255, 255, 255).
// This guards the cross-component reactive path (section write -> shared state -> mounted
// terminal re-render), which a per-component setting read silently broke.

// The Terminal scheme control is a shadcn/base-ui Select (a button trigger + a portaled
// listbox), not a native <select> -- open it and click the option by its visible text via a
// genuine WebDriver click (see settings-theme.test.ts for why a synthetic click is unreliable).
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

function terminalBackground(b: Browser): Promise<string> {
  return b.execute(() => {
    const el = document.querySelector("[data-surface-id]");
    return el ? getComputedStyle(el).backgroundColor : "";
  });
}

// The settings editor is a route: opening it unmounts the session's terminal panel, so the
// truly-live (mounted xterm re-themes in place) half of the scheme contract is proven by the
// useLiveTerminalTheme unit tests. What only a real webview can prove is the full round trip:
// the scheme picked in the editor reaches the terminal that mounts when the session returns.
async function setScheme(b: Browser, sessionUrl: string, scheme: string): Promise<void> {
  await (await b.$('[aria-label="Settings"]')).click();
  await (await b.$('[data-testid="settings-editor"]')).waitForExist({ timeout: 10_000 });
  await (await b.$('[data-testid="settings-section-terminal"]')).click();
  await (await b.$('[aria-label="Terminal scheme"]')).waitForExist({ timeout: 10_000 });
  await selectOption(b, "Terminal scheme", scheme);
  await b.execute((u: string) => {
    window.history.pushState({}, "", new URL(u).pathname);
    window.dispatchEvent(new Event("popstate"));
  }, sessionUrl);
  await (await b.$("[data-surface-id]")).waitForExist({ timeout: 10_000 });
}

test("changing the terminal scheme re-themes a mounted terminal live", async () => {
  const b = getApp();

  await createProject(b, "scheme-e2e");
  await openTerminal(b);
  const sessionUrl = await b.getUrl();

  // Establish a known scheme (state-independent), then flip it; the terminal that renders on
  // the session route reflects each pick. dark = rgb(13, 17, 23), light = rgb(255, 255, 255).
  await setScheme(b, sessionUrl, "github-dark");
  await b.waitUntil(async () => (await terminalBackground(b)) === "rgb(13, 17, 23)", {
    timeout: 15_000,
    timeoutMsg: "setting the dark scheme did not reach the mounted terminal",
  });

  await setScheme(b, sessionUrl, "github-light");
  await b.waitUntil(async () => (await terminalBackground(b)) === "rgb(255, 255, 255)", {
    timeout: 10_000,
    timeoutMsg: "terminal scheme change did not reach the mounted terminal",
  });
  expect(await terminalBackground(b)).toBe("rgb(255, 255, 255)");
}, 120_000);
