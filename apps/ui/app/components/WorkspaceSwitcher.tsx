import { useState, useCallback, useEffect } from "react";
import { FolderPlus, ArrowUpRight } from "lucide-react";
import { useDesktopHost } from "~/lib/useDesktopHost";
import { SessionSidebar } from "~/components/SessionSidebar";
import { cn } from "~/lib/utils";
import {
  focusSelf,
  focusWindow,
  onReattachWorkspace,
  openWindow,
  workspaceLabel,
  workspaceQuery,
} from "~/lib/windows";
import type { Workspace } from "@tillerd/sdk/orchestrator";

// ── WorkspaceSwitcher (pure / testable) ───────────────────────────────────────

/** Props for the presentational workspace switcher strip. */
export interface WorkspaceSwitcherProps {
  workspaces: Workspace[];
  activeId: string | null;
  /** Ids of workspaces currently detached to their own window (show the focus affordance). */
  detachedIds: Set<string>;
  isDesktop: boolean;
  onSelect: (id: string) => void;
  onNewWorkspace: () => void;
  onDetach: (id: string) => void;
  onFocusDetached: (id: string) => void;
}

/** Presentational list of workspace chips + new-workspace control — no data fetching. */
export function WorkspaceSwitcherList({
  workspaces,
  activeId,
  detachedIds,
  isDesktop,
  onSelect,
  onNewWorkspace,
  onDetach,
  onFocusDetached,
}: WorkspaceSwitcherProps) {
  return (
    <div
      data-testid="workspace-switcher"
      className="flex flex-col gap-0.5 px-3 py-2 border-b border-border/40 shrink-0"
    >
      {workspaces.map((ws) => (
        <div key={ws.id} className="flex items-center gap-1">
          <button
            type="button"
            data-testid="workspace-item"
            data-workspace-id={ws.id}
            onClick={() => onSelect(ws.id)}
            className={cn(
              "flex-1 text-left text-[0.75rem] truncate px-2 py-0.5 rounded-sm transition-colors duration-[var(--motion-fast)] ease-standard",
              ws.id === activeId
                ? "font-medium bg-muted text-foreground"
                : "text-muted-foreground hover:text-foreground hover:bg-muted",
            )}
          >
            {ws.name}
          </button>
          {detachedIds.has(ws.id) ? (
            <button
              type="button"
              onClick={() => onFocusDetached(ws.id)}
              aria-label={`Focus ${ws.name} window`}
              title={`${ws.name} is open in another window`}
              data-testid="workspace-detached-indicator"
              data-workspace-id={ws.id}
              className={cn(
                "flex items-center p-0.5 rounded-sm transition-colors duration-[var(--motion-fast)] ease-standard",
                "text-amber-500/80 hover:text-amber-400 hover:bg-muted",
              )}
            >
              <ArrowUpRight size={10} strokeWidth={2} />
            </button>
          ) : (
            isDesktop && (
              <button
                type="button"
                onClick={() => onDetach(ws.id)}
                aria-label={`Open ${ws.name} in a new window`}
                title={`Open ${ws.name} in a new window`}
                data-testid="workspace-detach"
                data-workspace-id={ws.id}
                className={cn(
                  "flex items-center p-0.5 rounded-sm transition-colors duration-[var(--motion-fast)] ease-standard",
                  "text-muted-foreground/50 hover:text-foreground hover:bg-muted",
                )}
              >
                <ArrowUpRight size={10} strokeWidth={2} />
              </button>
            )
          )}
        </div>
      ))}
      {isDesktop && (
        <button
          type="button"
          onClick={onNewWorkspace}
          data-testid="new-workspace"
          className={cn(
            "flex items-center gap-1.5 px-2 h-6 text-[0.75rem] rounded-sm transition-colors duration-[var(--motion-fast)] ease-standard",
            "text-muted-foreground hover:text-foreground hover:bg-muted",
          )}
          title="New workspace"
        >
          <FolderPlus size={11} strokeWidth={2} />
          <span>New workspace</span>
        </button>
      )}
    </div>
  );
}

// ── WorkspaceSwitcher (connected) ─────────────────────────────────────────────

/**
 * Connected wrapper: fetches workspace list, manages active workspace state,
 * and renders the sidebar scoped to the selected workspace.
 * Drop-in replacement for SessionSidebar when workspace support is needed.
 */
export function WorkspaceSwitcher({ initialWorkspaceId }: { initialWorkspaceId?: string } = {}) {
  const host = useDesktopHost();
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [activeWorkspaceId, setActiveWorkspaceId] = useState<string | null>(
    initialWorkspaceId ?? null,
  );
  const [detachedWorkspaces, setDetachedWorkspaces] = useState<Set<string>>(() => new Set());

  const isDesktop = host.status === "ready";

  const refresh = useCallback(async () => {
    if (host.status !== "ready") return;
    try {
      const list = await host.orchestratorClient.listWorkspaces();
      setWorkspaces(list);
      // Preserve current selection if still valid; otherwise reset to null (show all).
      setActiveWorkspaceId((prev) => (list.some((w) => w.id === prev) ? prev : null));
    } catch {
      // non-fatal; keep stale data
    }
  }, [host]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  // A re-attached child window emits this event; clear the detach flag and refocus the parent.
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void onReattachWorkspace(({ workspaceId }) => {
      setDetachedWorkspaces((prev) => {
        if (!prev.has(workspaceId)) return prev;
        const next = new Set(prev);
        next.delete(workspaceId);
        return next;
      });
      void focusSelf();
    }).then((u) => (unlisten = u));
    return () => unlisten?.();
  }, []);

  const handleNewWorkspace = useCallback(async () => {
    if (host.status !== "ready") return;
    // window.prompt is unreliable in the Tauri webview (often returns null with no dialog), so a
    // cancelled/empty result still creates a workspace under a default name — matching "New project".
    const name = window.prompt("Workspace name:")?.trim() || "New workspace";
    const ws = await host.orchestratorClient.createWorkspace({ name });
    await refresh();
    setActiveWorkspaceId(ws.id);
  }, [host, refresh]);

  const handleDetach = useCallback((workspaceId: string) => {
    void openWindow(workspaceLabel(workspaceId), workspaceQuery(workspaceId));
    setDetachedWorkspaces((prev) => new Set(prev).add(workspaceId));
  }, []);

  const handleFocusDetached = useCallback((workspaceId: string) => {
    void focusWindow(workspaceLabel(workspaceId));
  }, []);

  return (
    <div className="flex flex-col h-full">
      <WorkspaceSwitcherList
        workspaces={workspaces}
        activeId={activeWorkspaceId}
        detachedIds={detachedWorkspaces}
        isDesktop={isDesktop}
        onSelect={setActiveWorkspaceId}
        onNewWorkspace={() => void handleNewWorkspace()}
        onDetach={handleDetach}
        onFocusDetached={handleFocusDetached}
      />
      <SessionSidebar activeWorkspaceId={activeWorkspaceId ?? undefined} />
    </div>
  );
}
