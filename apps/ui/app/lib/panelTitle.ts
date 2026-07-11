import { formatElapsed } from "~/lib/formatElapsed";

// Panel title content (ui-panel-compound "Panel title content"): session name + surface kind,
// plus elapsed time since the surface's PTY spawned once known. Segments join with a middle-dot
// separator; an empty segment is dropped so a missing name never leaves a dangling separator.
export function terminalTitle(sessionName: string, spawnedAt: number | null, now: number): string {
  const segments = [sessionName, "Terminal"];
  if (spawnedAt != null) segments.push(formatElapsed(spawnedAt, now));
  return segments.filter((s) => s.length > 0).join(" · ");
}

// The session's display name for a panel title: a blank or whitespace-only title falls back to a
// short id slice (mirroring the sidebar's `title || id.slice(0, 8)`), then to a generic label.
export function sessionDisplayName(
  sessionTitle: string | undefined,
  sessionId: string | null,
): string {
  return sessionTitle?.trim() || sessionId?.slice(0, 8) || "Session";
}
