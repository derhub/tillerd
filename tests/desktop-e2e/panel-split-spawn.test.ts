import { expect, test } from "bun:test";

import { createProject, openTerminal, splitPanel, surfaceIds, uniqueName } from "./helpers";
import { getApp } from "./shared-app";

// Panel split + spawn (ui-panel-compound spec): splitting a live terminal panel produces a
// second, empty leaf that shows the surface-kind picker rather than auto-spawning; picking
// "terminal" spawns a second, independent PTY into that leaf.

test("splitting a terminal panel and picking a kind spawns a second, independent surface", async () => {
  const b = getApp();
  await createProject(b, uniqueName("Split"));
  const first = await openTerminal(b);
  expect(first).toBeTruthy();

  await splitPanel(b, "right");

  // The new leaf starts empty -- its picker, not a second live surface, is what appears. Splitting
  // re-parents the original leaf into a new group node, which remounts its terminal pane (the
  // surface briefly disappears and reconnects to the same placement), so poll rather than assert
  // on a single snapshot.
  const picker = await b.$('[data-testid="empty-panel-picker"]');
  await picker.waitForExist({ timeout: 10_000 });
  await b.waitUntil(async () => (await surfaceIds(b)).length === 1, {
    timeout: 10_000,
    timeoutMsg: "the original surface did not remain mounted after the split",
  });

  const terminalKind = await b.$('[data-testid="empty-panel-kind-terminal"]');
  await terminalKind.waitForExist({ timeout: 10_000 });
  await terminalKind.click();

  await b.waitUntil(async () => (await surfaceIds(b)).length === 2, {
    timeout: 20_000,
    timeoutMsg: "second surface did not mount after spawning into the split leaf",
  });

  // Two live surfaces, each its own PTY -- not the same surface rendered twice.
  const ids = await surfaceIds(b);
  expect(new Set(ids).size).toBe(2);
  expect(ids).toContain(first);
}, 120_000);
