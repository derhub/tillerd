// Multi-window plumbing (roadmap 0.0.11). A child window is another webview of the same app and
// backend; it carries its intent in the URL query (`?w=detached|project`) the shell reads, because
// the custom scheme has no SPA fallback for a deep route. Window labels are the cross-window
// identity used by the host `window_open` / `window_focus` commands.

import { isDesktopHost, loadTauriCore } from "./transport/core";

export type WindowIntent =
  | { kind: "main" }
  | { kind: "detached"; sessionId: string; placement: string }
  | { kind: "project"; projectId: string; sessionId: string | null };

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
    default:
      return { kind: "main" };
  }
}

async function invoke<T>(cmd: string, args: Record<string, unknown>): Promise<T | null> {
  if (!isDesktopHost()) return null;
  const core = await loadTauriCore();
  return core.invoke(cmd, args) as Promise<T>;
}

export function openWindow(label: string, query: string): Promise<void | null> {
  return invoke<void>("window_open", { label, query });
}

export function focusWindow(label: string): Promise<void | null> {
  return invoke<void>("window_focus", { label });
}

// ── Cross-window re-attach event contract ────────────────────────────────────

const REATTACH_PANEL = "panel:reattach";
const REATTACH_PROJECT = "project:reattach";

export type ReattachPanel = { sessionId: string; placement: string };
export type ReattachProject = { projectId: string };

async function emit(event: string, payload: unknown): Promise<void> {
  if (!isDesktopHost()) return;
  const { emit } = await import("@tauri-apps/api/event");
  await emit(event, payload);
}

async function listen<T>(event: string, cb: (payload: T) => void): Promise<() => void> {
  if (!isDesktopHost()) return () => {};
  const { listen } = await import("@tauri-apps/api/event");
  return listen<T>(event, (e) => cb(e.payload));
}

export const emitReattachPanel = (p: ReattachPanel) => emit(REATTACH_PANEL, p);
export const emitReattachProject = (p: ReattachProject) => emit(REATTACH_PROJECT, p);
export const onReattachPanel = (cb: (p: ReattachPanel) => void) => listen(REATTACH_PANEL, cb);
export const onReattachProject = (cb: (p: ReattachProject) => void) => listen(REATTACH_PROJECT, cb);

// Focus the window this code runs in (the parent, when re-attaching back to it).
export async function focusSelf(): Promise<void> {
  if (!isDesktopHost()) return;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  await getCurrentWindow().setFocus();
}

// Arm a child window so ANY close path (native button or `closeSelf`) first emits its re-attach
// event, then destroys the window — the parent restores on that event. Returns an unlisten.
export async function armReattachOnClose(emitReattach: () => Promise<void>): Promise<() => void> {
  if (!isDesktopHost()) return () => {};
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  const win = getCurrentWindow();
  return win.onCloseRequested(async (event) => {
    event.preventDefault();
    await emitReattach();
    await win.destroy();
  });
}

// Request this window to close, routing through the armed re-attach handler.
export async function closeSelf(): Promise<void> {
  if (!isDesktopHost()) return;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  await getCurrentWindow().close();
}
