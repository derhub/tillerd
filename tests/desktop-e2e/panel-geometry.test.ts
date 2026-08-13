import { expect, test } from "bun:test";

import {
  createProject,
  openTerminal,
  openView,
  panelIdForSurface,
  splitPanel,
  surfaceIds,
  uniqueName,
  type Browser,
} from "./helpers";
import { getApp } from "./shared-app";

type Rect = { left: number; top: number; width: number; height: number };
type Geometry = {
  id: string;
  rect: Rect;
  children: Array<{ id: string; rect: Rect }>;
  handle: Rect | null;
  axis: "x" | "y";
};

type LayoutProbeWindow = Window & {
  __e2eLayoutRead?: unknown;
  __e2eLayoutSet?: unknown;
};

function sessionId(url: string): string {
  const match = /\/session\/([^/?#]+)/.exec(url);
  if (!match) throw new Error(`session route missing from ${url}`);
  return decodeURIComponent(match[1]!);
}

async function setStoredLayout(b: Browser, id: string, layoutJson: string): Promise<void> {
  await b.execute(
    (session: string, raw: string) => {
      const w = window as LayoutProbeWindow;
      const internals = w as unknown as {
        __TAURI_INTERNALS__: { invoke(command: string, args: unknown): Promise<unknown> };
      };
      w.__e2eLayoutSet = false;
      void internals.__TAURI_INTERNALS__
        .invoke("session_layout_set", { id: session, layoutJson: raw })
        .then(() => {
          w.__e2eLayoutSet = true;
        })
        .catch(() => {
          w.__e2eLayoutSet = "error";
        });
    },
    id,
    layoutJson,
  );
  await b.waitUntil(
    async () => (await b.execute(() => (window as LayoutProbeWindow).__e2eLayoutSet)) === true,
    { timeout: 10_000, timeoutMsg: "session layout write did not complete" },
  );
}

async function readStoredLayout(b: Browser, id: string): Promise<string | null> {
  await b.execute((session: string) => {
    const w = window as LayoutProbeWindow;
    const internals = w as unknown as {
      __TAURI_INTERNALS__: { invoke(command: string, args: unknown): Promise<unknown> };
    };
    w.__e2eLayoutRead = undefined;
    void internals.__TAURI_INTERNALS__
      .invoke("session_layout_get", { id: session })
      .then((value) => {
        w.__e2eLayoutRead = value;
      })
      .catch(() => {
        w.__e2eLayoutRead = "error";
      });
  }, id);
  await b.waitUntil(
    async () =>
      (await b.execute(() => (window as LayoutProbeWindow).__e2eLayoutRead)) !== undefined,
    { timeout: 10_000, timeoutMsg: "session layout read did not complete" },
  );
  const value = await b.execute(() => (window as LayoutProbeWindow).__e2eLayoutRead);
  if (value === "error" || value === null) return null;
  if (typeof value !== "string")
    throw new Error(`unexpected session layout value: ${JSON.stringify(value)}`);
  return value;
}

async function geometries(b: Browser): Promise<Geometry[]> {
  return b.execute(() => {
    const rectOf = (element: Element): Rect => {
      const rect = element.getBoundingClientRect();
      return { left: rect.left, top: rect.top, width: rect.width, height: rect.height };
    };
    return Array.from(
      document.querySelectorAll<HTMLElement>('[data-slot="resizable-panel-group"][id]'),
    ).map((group) => {
      const children = Array.from(group.children)
        .filter((child) => child.getAttribute("data-slot") === "resizable-panel")
        .map((child) => ({
          id: child.getAttribute("data-panel-node-id") ?? child.id,
          rect: rectOf(child),
        }));
      const handles = Array.from(group.children).filter(
        (child) => child.getAttribute("data-slot") === "resizable-handle",
      );
      const first = children[0]?.rect;
      const second = children[1]?.rect;
      const axis =
        first && second && Math.abs(first.top - second.top) > Math.abs(first.left - second.left)
          ? "y"
          : "x";
      return {
        id: group.id,
        rect: rectOf(group),
        children,
        handle: handles[0] ? rectOf(handles[0]) : null,
        axis,
      };
    });
  });
}

function geometryFor(geometry: Geometry[], childId: string): Geometry {
  const group = geometry.find((candidate) =>
    candidate.children.some((child) => child.id === childId),
  );
  if (!group) throw new Error(`split group containing ${childId} was not found`);
  return group;
}

function share(group: Geometry, index: number): number {
  const child = group.children[index];
  if (!child) throw new Error(`group ${group.id} has no child ${index}`);
  const length = group.axis === "x" ? group.rect.width : group.rect.height;
  return (group.axis === "x" ? child.rect.width : child.rect.height) / length;
}

async function resizeDivider(b: Browser, group: Geometry): Promise<void> {
  if (!group.handle) throw new Error(`group ${group.id} has no resize handle`);
  const handle = b.$(`[id="${group.id}"] > [data-slot="resizable-handle"]`);
  await handle.click();
  const key = group.axis === "x" ? "ArrowRight" : "ArrowDown";
  await b.keys([key, key]);
}

test("nested divider resize persists independently across reload", async () => {
  const b = getApp();
  await openView(b, "Sessions");
  await createProject(b, uniqueName("Geometry"));
  await openTerminal(b);
  await splitPanel(b, "right");
  await (await b.$('[data-testid="empty-panel-kind-terminal"]')).click();
  await b.waitUntil(async () => (await surfaceIds(b)).length === 2, {
    timeout: 20_000,
    timeoutMsg: "root split did not mount two terminals",
  });

  const surfaces = await surfaceIds(b);
  const nestedTarget = await panelIdForSurface(b, surfaces[1]!);
  await (await b.$(`[data-panel-id="${nestedTarget}"] button[aria-label="Split down"]`)).click();
  await (await b.$('[data-testid="empty-panel-kind-terminal"]')).click();
  await b.waitUntil(async () => (await surfaceIds(b)).length === 3, {
    timeout: 20_000,
    timeoutMsg: "nested split did not mount a third terminal",
  });

  const before = await geometries(b);
  const nested = geometryFor(before, nestedTarget);
  const root = before.find((group) => group.children.some((child) => child.id === nested.id));
  if (!root) throw new Error("root split geometry was not found");
  const rootId = root.id;
  const rootShare = share(root, 0);
  const nestedShare = share(nested, 0);
  const beforeLayout = await readStoredLayout(b, sessionId(await b.getUrl()));
  expect(beforeLayout).toBeTruthy();

  await resizeDivider(b, nested);
  await b.waitUntil(
    async () => {
      const current = geometryFor(await geometries(b), nestedTarget);
      return share(current, 0) > nestedShare + 0.08;
    },
    { timeout: 10_000, timeoutMsg: "nested divider did not resize" },
  );

  const after = await geometries(b);
  const resizedNested = geometryFor(after, nestedTarget);
  const resizedRoot = after.find((group) => group.id === rootId);
  if (!resizedRoot) throw new Error("root split disappeared after nested resize");
  expect(share(resizedNested, 0)).toBeGreaterThan(nestedShare + 0.08);
  expect(Math.abs(share(resizedRoot, 0) - rootShare)).toBeLessThan(0.05);

  const id = sessionId(await b.getUrl());
  await b.waitUntil(
    async () => {
      const raw = await readStoredLayout(b, id);
      return raw !== null && raw !== beforeLayout;
    },
    { timeout: 10_000, timeoutMsg: "resized geometry did not persist" },
  );

  await b.refresh();
  await b.waitUntil(async () => (await b.$("body").getText()).includes("services: ready"), {
    timeout: 45_000,
    timeoutMsg: "app did not reach ready after geometry reload",
  });
  await b.waitUntil(
    async () => {
      const restored = await geometries(b);
      return (
        restored.some((group) => group.id === rootId) &&
        restored.some((group) => group.children.some((child) => child.id === nestedTarget))
      );
    },
    {
      timeout: 20_000,
      timeoutMsg: "nested split did not restore after reload",
    },
  );
  const restored = await geometries(b);
  const restoredNested = geometryFor(restored, nestedTarget);
  const restoredRoot = restored.find((group) => group.id === rootId);
  if (!restoredRoot) throw new Error("root split did not restore after reload");
  expect(Math.abs(share(restoredNested, 0) - share(resizedNested, 0))).toBeLessThan(0.05);
  expect(Math.abs(share(restoredRoot, 0) - share(resizedRoot, 0))).toBeLessThan(0.05);
}, 120_000);

test("unversioned session layout reports an incompatible layout without replacement", async () => {
  const b = getApp();
  await openView(b, "Sessions");
  await createProject(b, uniqueName("Legacy layout"));
  const id = sessionId(await b.getUrl());
  const legacy = JSON.stringify({
    kind: "panel",
    id: "legacy-root",
    title: "Empty",
    content: { type: "empty" },
  });
  await setStoredLayout(b, id, legacy);
  await b.refresh();
  await b.waitUntil(async () => (await b.$("body").getText()).includes("services: ready"), {
    timeout: 45_000,
    timeoutMsg: "app did not reach ready after incompatible layout reload",
  });

  const alert = await b.$('[role="alert"]');
  await alert.waitForExist({ timeout: 15_000 });
  expect(await alert.getText()).toContain("This session layout is incompatible");
  expect(await b.$("button*=New terminal").isExisting()).toBe(false);
  expect(await b.$("aside").isExisting()).toBe(true);
  expect(await readStoredLayout(b, id)).toBe(legacy);
}, 120_000);
