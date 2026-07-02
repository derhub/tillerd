// Child windows are webviews of the same app+backend; intent is carried in the URL query
// (`?w=detached|project`) because the custom scheme has no SPA fallback for a deep route.

import { windowClose, windowFocus, windowOpen } from "@tillerd/client-bindings";

import { currentWindow, emitEvent, listenEvent } from "./tauriEvents";
import { isDesktopHost } from "./transport/core";

export type WindowIntent =
  | { kind: "main" }
  | { kind: "detached"; sessionId: string; placement: string }
  | { kind: "project"; projectId: string; sessionId: string | null }
  | { kind: "workspace"; workspaceId: string };

export function detachedLabel(placement: string): string {
  return `detached-${placement}`;
}

export function projectLabel(projectId: string): string {
  return `project-${projectId}`;
}

export function detachedQuery(sessionId: string, placement: string): string {
  return `?${new URLSearchParams({ w: "detached", session: sessionId, placement }).toString()}`;
}

export function projectQuery(projectId: string, sessionId: string | null): string {
  const params = new URLSearchParams({ w: "project", project: projectId });
  if (sessionId) params.set("session", sessionId);
  return `?${params.toString()}`;
}

export function workspaceLabel(workspaceId: string): string {
  return `workspace-${workspaceId}`;
}

export function workspaceQuery(workspaceId: string): string {
  return `?${new URLSearchParams({ w: "workspace", workspace: workspaceId }).toString()}`;
}

export function parseWindowIntent(search: string): WindowIntent {
  const params = new URLSearchParams(search);
  switch (params.get("w")) {
    case "detached": {
      const sessionId = params.get("session");
      const placement = params.get("placement");
      if (sessionId && placement) return { kind: "detached", sessionId, placement };
      return { kind: "main" };
    }
    case "project": {
      const projectId = params.get("project");
      if (projectId) return { kind: "project", projectId, sessionId: params.get("session") };
      return { kind: "main" };
    }
    case "workspace": {
      const workspaceId = params.get("workspace");
      if (workspaceId) return { kind: "workspace", workspaceId };
      return { kind: "main" };
    }
    default:
      return { kind: "main" };
  }
}

export function openWindow(label: string, query: string): Promise<void | null> {
  return isDesktopHost() ? windowOpen(label, query) : Promise.resolve(null);
}

export function focusWindow(label: string): Promise<void | null> {
  return isDesktopHost() ? windowFocus(label) : Promise.resolve(null);
}

// Closing a child via this call triggers armReattachOnClose on the child, which emits the re-attach event.
export function closeWindow(label: string): Promise<void | null> {
  return isDesktopHost() ? windowClose(label) : Promise.resolve(null);
}

const REATTACH_PANEL = "panel:reattach";
const REATTACH_PROJECT = "project:reattach";
const REATTACH_WORKSPACE = "workspace:reattach";

export type ReattachPanel = { sessionId: string; placement: string };
export type ReattachProject = { projectId: string };
export type ReattachWorkspace = { workspaceId: string };

export const emitReattachPanel = (p: ReattachPanel) => emitEvent(REATTACH_PANEL, p);
export const emitReattachProject = (p: ReattachProject) => emitEvent(REATTACH_PROJECT, p);
export const emitReattachWorkspace = (p: ReattachWorkspace) => emitEvent(REATTACH_WORKSPACE, p);
export const onReattachPanel = (cb: (p: ReattachPanel) => void) => listenEvent(REATTACH_PANEL, cb);
export const onReattachProject = (cb: (p: ReattachProject) => void) =>
  listenEvent(REATTACH_PROJECT, cb);
export const onReattachWorkspace = (cb: (p: ReattachWorkspace) => void) =>
  listenEvent(REATTACH_WORKSPACE, cb);

export async function focusSelf(): Promise<void> {
  await (await currentWindow())?.setFocus();
}

// Any close path (native button or closeSelf) emits the re-attach event before destroying the window.
export async function armReattachOnClose(emitReattach: () => Promise<void>): Promise<() => void> {
  const win = await currentWindow();
  if (!win) return () => {};
  return win.onCloseRequested(async (event) => {
    event.preventDefault();
    await emitReattach();
    await win.destroy();
  });
}

export async function closeSelf(): Promise<void> {
  await (await currentWindow())?.close();
}
