import { expect, test } from "bun:test";

import { createProject, openTerminal, panelIdForSurface, splitPanel, uniqueName } from "./helpers";
import { getApp } from "./shared-app";

// Panel placement swap via drag/drop (panel-placement-swap spec): dragging one terminal panel's
// header onto another swaps which surface backs each panel WITHOUT moving the panels themselves
// (geometry is unchanged -- only the surface binding flips). tauri-webdriver drives a single real
// WKWebView, not a synthetic test DOM, so this dispatches genuine `DragEvent`s carrying a real
// `DataTransfer` -- WebdriverIO's own drag-and-drop helpers assume a Chromium/Firefox W3C actions
// backend tauri-webdriver does not implement. A `DataTransfer` built via `new DataTransfer()` and
// reused across dragstart/dragover/drop (not a browser-tracked native drag) stays read/write for
// the whole sequence in WebKit, so `getData` on drop works the same as `setData` on dragstart --
// this DID work in practice (see the assertions below), so no wire-driven fallback was needed.

test("dragging one panel's header onto another swaps their surfaces, not their positions", async () => {
  const b = getApp();
  await createProject(b, uniqueName("SwapDnd"));

  const s1 = await openTerminal(b);
  await splitPanel(b, "right");
  const kind = await b.$('[data-testid="empty-panel-kind-terminal"]');
  await kind.waitForExist({ timeout: 10_000 });
  await kind.click();
  await b.waitUntil(async () => (await b.$$("[data-surface-id]")).length === 2, {
    timeout: 20_000,
    timeoutMsg: "second surface did not mount before the swap",
  });

  const panelA = await panelIdForSurface(b, s1); // the leaf s1 is currently in
  const allSurfaces = await b.execute(() =>
    Array.from(document.querySelectorAll("[data-surface-id]"), (el) =>
      el.getAttribute("data-surface-id"),
    ),
  );
  const s2 = allSurfaces.find((id) => id !== s1);
  expect(s2).toBeTruthy();
  const panelB = await panelIdForSurface(b, s2 as string);
  expect(panelA).not.toBe(panelB);

  // dragstart on panel A's draggable header, dragover on panel B's frame -- reusing one
  // DataTransfer across both, exactly as a real drag would carry one across its whole gesture.
  await b.execute(
    (sourcePanelId: string, targetPanelId: string) => {
      const header = document.querySelector(
        `[data-panel-id="${sourcePanelId}"] [draggable="true"]`,
      );
      const targetFrame = document.querySelector(`[data-panel-id="${targetPanelId}"]`);
      if (!header || !targetFrame) throw new Error("drag source or target not found in the DOM");
      const dt = new DataTransfer();
      (window as unknown as { __e2eDragDt: DataTransfer }).__e2eDragDt = dt;
      header.dispatchEvent(
        new DragEvent("dragstart", { bubbles: true, cancelable: true, dataTransfer: dt }),
      );
      targetFrame.dispatchEvent(
        new DragEvent("dragover", { bubbles: true, cancelable: true, dataTransfer: dt }),
      );
    },
    panelA,
    panelB,
  );

  // The hover highlight (panel-drop-target-active) is the drop target's own anchor, only present
  // while a compatible drag is over it.
  const highlighted = await b.$(
    `[data-panel-id="${panelB}"][data-testid="panel-drop-target-active"]`,
  );
  await highlighted.waitForExist({
    timeout: 10_000,
    timeoutMsg: "drop-target highlight did not appear on dragover",
  });

  await b.execute(
    (sourcePanelId: string, targetPanelId: string) => {
      const dt = (window as unknown as { __e2eDragDt: DataTransfer }).__e2eDragDt;
      const header = document.querySelector(
        `[data-panel-id="${sourcePanelId}"] [draggable="true"]`,
      );
      const targetFrame = document.querySelector(`[data-panel-id="${targetPanelId}"]`);
      targetFrame?.dispatchEvent(
        new DragEvent("drop", { bubbles: true, cancelable: true, dataTransfer: dt }),
      );
      header?.dispatchEvent(
        new DragEvent("dragend", { bubbles: true, cancelable: true, dataTransfer: dt }),
      );
      delete (window as unknown as { __e2eDragDt?: DataTransfer }).__e2eDragDt;
    },
    panelA,
    panelB,
  );

  // The highlight clears once the drop lands.
  await highlighted.waitForExist({ timeout: 10_000, reverse: true });

  // Both PTYs are still alive (two surfaces total)...
  await b.waitUntil(async () => (await b.$$("[data-surface-id]")).length === 2, {
    timeout: 10_000,
    timeoutMsg: "a surface disappeared across the swap",
  });
  // ...but each panel's own slot now streams the OTHER surface -- the swap flips content, not
  // container position.
  await b.waitUntil(
    async () => {
      const nowInA = await panelIdForSurface(b, s2 as string);
      const nowInB = await panelIdForSurface(b, s1);
      return nowInA === panelA && nowInB === panelB;
    },
    { timeout: 15_000, timeoutMsg: "surfaces did not swap panels after the drop" },
  );
}, 120_000);
