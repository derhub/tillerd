import { test } from "bun:test";

import { type Browser } from "./helpers";
import { getApp } from "./shared-app";

// The command center is a leader-activated fuzzy palette over the chrome actions. The native leader
// accelerator is not WebDriver-reachable (native menu lives outside the webview), so these drive the
// renderer half by emitting the same `command-center:open` signal the accelerator produces, then
// assert listing, fuzzy filtering, invocation, and that a keybinding-preset change persists across a
// reload. The native accelerator registration is covered by a Rust host test; the resolution/merge
// logic by unit tests.

function openPalette(b: Browser): Promise<void> {
  return b.execute(() => window.dispatchEvent(new CustomEvent("command-center:open")));
}

// Right after a reload, "services: ready" can render before React finishes mounting
// CommandCenter and attaching its `command-center:open` listener (confirmed via a
// repeated diagnostic run: the plain dispatch reliably missed every attached listener in
// that window). A single dispatch is a no-op with no listener attached, so redispatching
// on a short interval until the palette actually appears is safe and self-healing.
async function openPaletteRetrying(b: Browser, timeoutMs: number): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  for (;;) {
    await openPalette(b);
    const opened = await (
      await b.$('[data-testid="command-center"]')
    )
      .waitForExist({ timeout: 300 })
      .then(() => true)
      .catch(() => false);
    if (opened) return;
    if (Date.now() >= deadline) {
      throw new Error(`command center did not open within ${timeoutMs}ms of reload`);
    }
  }
}

// A single WebDriver click can still land while the popover is mid-open/mid-animation and
// miss (confirmed: still occasionally left open after one click in a repeated diagnostic
// run). Re-clicking is harmless once already closed (body click with nothing to dismiss),
// so retry until the target element is actually gone.
async function closeBySelectorGone(b: Browser, selector: string, timeoutMs: number): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  for (;;) {
    await (await b.$("body")).click();
    const gone = await (
      await b.$(selector)
    )
      .waitForExist({ timeout: 300, reverse: true })
      .then(() => true)
      .catch(() => false);
    if (gone) return;
    if (Date.now() >= deadline) {
      throw new Error(`${selector} did not close within ${timeoutMs}ms`);
    }
  }
}

// Set a React controlled <select> and fire `change` -- webdriverio selectBy* helpers do not reliably
// trigger React onChange under WKWebView.
function selectValue(b: Browser, ariaLabel: string, value: string): Promise<void> {
  return b.execute(
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

// The "New terminal" command's rendered key hint, or "" when the item or hint is absent.
function newTerminalHint(b: Browser): Promise<string> {
  return b.execute(() => {
    const items = Array.from(document.querySelectorAll('[data-slot="command-item"]'));
    const item = items.find((el) => el.textContent?.includes("New terminal"));
    return item?.querySelector('[data-slot="command-shortcut"]')?.textContent ?? "";
  });
}

test("the palette opens, fuzzy-filters, and invokes an action", async () => {
  const b = getApp();

  await openPalette(b);
  const palette = await b.$('[data-testid="command-center"]');
  await palette.waitForExist({ timeout: 10_000 });

  // Lists chrome actions.
  await (await b.$("div*=View logs")).waitForExist({ timeout: 10_000 });

  // Fuzzy filter to the logs action, then invoke it -- the same handler View > Logs runs.
  const input = await b.$('[data-testid="command-center-input"]');
  await input.click();
  await b.keys(Array.from("logs"));
  await b.waitUntil(
    async () => {
      const items = await b.$$('[data-slot="command-item"]');
      return items.length === 1;
    },
    { timeout: 10_000, timeoutMsg: "query did not narrow the palette to one match" },
  );
  await b.keys(["Enter"]);

  await b.waitUntil(async () => (await b.getUrl()).includes("/logs"), {
    timeout: 10_000,
    timeoutMsg: "invoking the logs action did not route to /logs",
  });
}, 120_000);

test("a keybinding-preset change reflects in the palette and survives a reload", async () => {
  const b = getApp();

  // Never assume the prior test left a clean slate: the palette and the settings popover
  // are two independent, un-coordinated `open` booleans (no code enforces mutual
  // exclusivity), so opening the second while the first is still up would stack two
  // overlays. Close the palette first if it is somehow still open.
  if (await (await b.$('[data-testid="command-center"]')).isExisting()) {
    await b.keys(["Escape"]);
    await (
      await b.$('[data-testid="command-center"]')
    ).waitForExist({
      timeout: 5_000,
      reverse: true,
    });
  }

  // Switch the preset to vscode through the settings panel (vscode binds New terminal to a backtick
  // chord; the default does not).
  await b.execute(() => window.dispatchEvent(new CustomEvent("command-center:settings")));
  await (await b.$('select[aria-label="Keybinding preset"]')).waitForExist({ timeout: 10_000 });
  await selectValue(b, "Keybinding preset", "vscode");
  // Close the settings popover before opening the palette. A synthetic MouseEvent
  // ("mousedown") never dismisses it: the popover's outside-press layer needs a real
  // pointer event sequence, and a synthetic Escape keypress is unreliable too (focus is
  // left on the just-changed native <select>, which can swallow it). A genuine WebDriver
  // click -- a full native pointerdown/up + mousedown/up + click sequence -- is what the
  // popover's dismiss layer needs; retried since a single click can still land mid-animation.
  await closeBySelectorGone(b, 'select[aria-label="Keybinding preset"]', 10_000);

  await openPalette(b);
  await (await b.$('[data-testid="command-center"]')).waitForExist({ timeout: 10_000 });
  await b.waitUntil(async () => (await newTerminalHint(b)).includes("`"), {
    timeout: 10_000,
    timeoutMsg: "preset change did not update the New terminal hint",
  });

  // The renderer persists settings fire-and-forget (`void source.setSetting`), so confirm the
  // write actually landed in the orchestrator before reloading -- otherwise the reload races
  // the async persist and the preset is lost (the source of this test's flakiness).
  //
  // `execute` returns null for an async browser function under tauri-webdriver, so the read
  // cannot be awaited inline. Each poll fires the invoke (fire-and-forget) into a window slot
  // and returns the slot's prior value synchronously; the slot reaches "vscode" once the write
  // is visible.
  try {
    await b.waitUntil(
      async () =>
        (await b.execute(() => {
          const w = window as unknown as {
            __TAURI_INTERNALS__: { invoke(cmd: string, args: unknown): Promise<unknown> };
            __presetPersisted?: unknown;
          };
          void w.__TAURI_INTERNALS__
            .invoke("setting_get", { scope: "global", projectId: null, key: "keybindings.preset" })
            .then((v) => {
              w.__presetPersisted = typeof v === "string" ? JSON.parse(v) : v;
            })
            .catch((err) => {
              w.__presetPersisted = "ERROR: " + (err?.message || err || "unknown");
            });
          return w.__presetPersisted ?? null;
        })) === "vscode",
      { timeout: 10_000 },
    );
  } catch {
    const val = await b.execute(() => (window as any).__presetPersisted);
    throw new Error(`preset did not persist to the orchestrator before reload. Last value: ${val}`);
  }

  // Reload the webview and confirm the preset re-applies after re-hydration. A WebDriver-level
  // refresh (not a JS window.location.reload()) matches reload-deep-route.test.ts's proven-stable
  // reload path.
  await b.refresh();
  await b.waitUntil(async () => (await b.$("body").getText()).includes("services: ready"), {
    timeout: 45_000,
    timeoutMsg: "app did not reach ready after reload",
  });
  await openPaletteRetrying(b, 10_000);
  // Post-reload, the renderer re-establishes IPC readiness from scratch before settings
  // hydration can even start its fetch. Two distinct issues here: a default-interval
  // waitUntil (~500ms) hammers execute() calls with no gap, starving the single-threaded
  // webview's event loop enough to stall the in-flight fetchQuery's continuation -- a
  // longer poll interval fixes that. Separately, "services: ready" reflects only the
  // orchestrator connection (useDesktopHost), not this unrelated settings query's own
  // freshness, so the hydration round trip can genuinely still be in flight after that
  // text appears -- give it the same order of headroom as the ready-wait above.
  await b.waitUntil(async () => (await newTerminalHint(b)).includes("`"), {
    timeout: 60_000,
    interval: 2_000,
    timeoutMsg: "preset did not persist across reload",
  });
}, 180_000);
