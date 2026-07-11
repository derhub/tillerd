import { expect, test } from "bun:test";

import {
  createProject,
  openTerminal,
  panelIdForSurface,
  splitPanel,
  surfaceIds,
  type Browser,
  uniqueName,
} from "./helpers";
import { getApp } from "./shared-app";

// Close-surface confirmation (ui-panel-compound spec): closing a leaf bound to a running
// terminal prompts before its PTY is terminated, unless "don't ask again" is set -- then later
// closes in the SAME app session skip the dialog. `Panel.CloseButton` only renders once a leaf
// has a sibling (`totalPanels > 1`), so each close below first splits to create one, then closes
// the terminal-bound leaf specifically (both leaves get a "Close panel" button once split, so the
// close is scoped by the terminal's own `data-panel-id`, not a bare label match).

async function closeTerminalPanel(b: Browser, surface: string): Promise<void> {
  // A split re-parents the leaf into a new group node, which remounts its terminal pane -- the
  // surface briefly disappears from the DOM and reconnects to the same placement/surface. Poll
  // past that instead of reading the panel id off a single, possibly-mid-remount snapshot.
  await b.waitUntil(async () => (await panelIdForSurface(b, surface)) !== "", {
    timeout: 10_000,
    timeoutMsg: `surface ${surface} was not a mounted panel after the split`,
  });
  const panelId = await panelIdForSurface(b, surface);
  const closeButton = await b.$(`[data-panel-id="${panelId}"] button[aria-label="Close panel"]`);
  await closeButton.waitForExist({ timeout: 10_000 });
  await closeButton.click();
}

test("closing a running terminal prompts for confirmation, and 'don't ask again' skips it for the rest of the session", async () => {
  const b = getApp();
  await createProject(b, uniqueName("CloseConfirm"));

  // First close: a sibling leaf makes the close button appear, and the terminal is running, so
  // the confirmation dialog must appear.
  const surface1 = await openTerminal(b);
  await splitPanel(b, "right");
  await closeTerminalPanel(b, surface1);

  const dialog = await b.$('[data-testid="close-confirm-dialog"]');
  await dialog.waitForExist({ timeout: 10_000 });

  await (await b.$('[data-testid="close-confirm-dont-ask"] [data-slot="checkbox"]')).click();
  await (await b.$('[data-testid="close-confirm-confirm"]')).click();

  await dialog.waitForExist({ timeout: 10_000, reverse: true });
  await b.waitUntil(async () => (await surfaceIds(b)).length === 0, {
    timeout: 10_000,
    timeoutMsg: "terminal surface was not closed after confirming",
  });

  // Second close: the "don't ask again" preference persists (in-session, global setting) --
  // splitting and closing another running terminal now proceeds with no dialog at all.
  const surface2 = await openTerminal(b);
  await splitPanel(b, "right");
  await closeTerminalPanel(b, surface2);

  await b.waitUntil(async () => (await surfaceIds(b)).length === 0, {
    timeout: 10_000,
    timeoutMsg: "second terminal did not close",
  });
  expect(await (await b.$('[data-testid="close-confirm-dialog"]')).isExisting()).toBe(false);
}, 120_000);
