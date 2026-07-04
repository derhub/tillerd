import { Store, useSelector } from "@tanstack/react-store";

import { surfaceStatusChannel, type SurfaceStatusChannelHandle } from "@tillerd/client-bindings";

import { isDesktopHost } from "./transport/core";

// Per-session surface-runtime badge derived from the surface-status push channel:
// the orchestrator pushes {sessionId, surfaceId, status} after each status write,
// so the badge tracks transitions without polling. Absent surfaces read as idle
// (no runtime yet); the sidebar only reflects state the channel has announced.

export type SessionBadge = "starting" | "running" | "failed" | "idle";

// Raw surface statuses per session, keyed surfaceId -> status, so a later
// transition of one surface does not clobber the others' known state.
type SurfaceMap = Record<string, string>;

const store = new Store<Record<string, SurfaceMap>>({});

// failed dominates, then a live surface (running), then a pending one (starting).
function aggregate(surfaces: SurfaceMap | undefined): SessionBadge {
  if (!surfaces) return "idle";
  let sawPending = false;
  let sawLive = false;
  for (const status of Object.values(surfaces)) {
    if (status === "failed") return "failed";
    if (status === "live") sawLive = true;
    else if (status === "pending") sawPending = true;
  }
  if (sawLive) return "running";
  if (sawPending) return "starting";
  return "idle";
}

function record(sessionId: string, surfaceId: string, status: string): void {
  store.setState((s) => {
    const current = s[sessionId] ?? {};
    if (current[surfaceId] === status) return s;
    return { ...s, [sessionId]: { ...current, [surfaceId]: status } };
  });
}

export function useSessionBadge(sessionId: string): SessionBadge {
  return useSelector(store, (s) => aggregate(s[sessionId]));
}

// Single per-window subscription; the sessions view mounts it. Off the desktop
// host (web preview) there is no channel, so nothing mounts.
export function mountSessionStatus(): () => void {
  if (!isDesktopHost()) return () => {};
  let handle: SurfaceStatusChannelHandle | undefined;
  let closed = false;
  void surfaceStatusChannel((event) => {
    record(event.sessionId, event.surfaceId, event.status);
  }).then((h) => {
    if (closed) void h.close();
    else handle = h;
  });
  return () => {
    closed = true;
    void handle?.close();
  };
}

// Test/reset hygiene.
export function resetSessionStatus(): void {
  store.setState(() => ({}));
}
