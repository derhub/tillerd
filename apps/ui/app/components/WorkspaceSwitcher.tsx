import { useState, useCallback, useEffect } from "react";
import { FolderPlus, ArrowUpRight } from "lucide-react";
import { useDesktopHost } from "~/lib/useDesktopHost";
import { InlineRenameInput } from "~/components/InlineRenameInput";
import { SessionSidebar } from "~/components/SessionSidebar";
import { cn } from "~/lib/utils";
import {
  closeWindow,
  focusSelf,
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
  /** Re-attach a detached workspace by closing its window (which fires the re-attach event). */
  onReattach: (id: string) => void;
  /** Id of the workspace whose name is being edited inline, or null. */
  editingId: string | null;
  onStartEdit: (id: string) => void;
  onCancelEdit: () => void;
  onRename: (id: string, name: string) => void;
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
  onReattach,
  editingId,
  onStartEdit,
  onCancelEdit,
  onRename,
}: WorkspaceSwitcherProps) {
  return (
    <div
      data-testid="workspace-switcher"
      className="flex flex-col gap-0.5 px-3 py-2 border-b border-border/40 shrink-0"
    >
      {workspaces.map((ws) => (
        <div key={ws.id} className="flex items-center gap-1">
          {editingId === ws.id ? (
            <InlineRenameInput
              initialValue={ws.name}
              onConfirm={(name) => onRename(ws.id, name)}
              onCancel={onCancelEdit}
            />
          ) : (
            <button
              type="button"
              data-testid="workspace-item"
              data-workspace-id={ws.id}
              onClick={() => onSelect(ws.id)}
              onDoubleClick={() => onStartEdit(ws.id)}
              className={cn(
                "flex-1 text-left text-[0.75rem] truncate px-2 py-0.5 rounded-sm transition-colors duration-[var(--motion-fast)] ease-standard",
                ws.id === activeId
                  ? "font-medium bg-muted text-foreground"
                  : "text-muted-foreground hover:text-foreground hover:bg-muted",
              )}
            >
              {ws.name}
            </button>
          )}
          {detachedIds.has(ws.id) ? (
            <button
              type="button"
              onClick={() => onReattach(ws.id)}
              aria-label={`Re-attach ${ws.name}`}
              title={`${ws.name} is in another window — click to re-attach`}
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
  const [editingId, setEditingId] = useState<string | null>(null);

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

  const handleRenameWorkspace = useCallback(
    async (id: string, newName: string) => {
      if (host.status !== "ready") return;
      await host.orchestratorClient.renameWorkspace({
        id,
        name: newName.trim() || "New workspace",
      });
      await refresh();
      setEditingId(null);
    },
    [host, refresh],
  );

  // Create under a placeholder name, then drop straight into inline rename. The Tauri webview has no
  // reliable text-input dialog (window.prompt returns null), so naming happens in-app, not via prompt.
  const handleNewWorkspace = useCallback(async () => {
    if (host.status !== "ready") return;
    const ws = await host.orchestratorClient.createWorkspace({ name: "New workspace" });
    await refresh();
    setActiveWorkspaceId(ws.id);
    setEditingId(ws.id);
  }, [host, refresh]);

  const handleDetach = useCallback((workspaceId: string) => {
    void openWindow(workspaceLabel(workspaceId), workspaceQuery(workspaceId));
    setDetachedWorkspaces((prev) => new Set(prev).add(workspaceId));
  }, []);

  const handleReattach = useCallback((workspaceId: string) => {
    void closeWindow(workspaceLabel(workspaceId));
    // Parent-initiated: clear the flag now rather than waiting for the child's re-attach event,
    // which may not fire if the child is closed before it armed its close handler.
    setDetachedWorkspaces((prev) => {
      if (!prev.has(workspaceId)) return prev;
      const next = new Set(prev);
      next.delete(workspaceId);
      return next;
    });
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
        onReattach={handleReattach}
        editingId={editingId}
        onStartEdit={setEditingId}
        onCancelEdit={() => setEditingId(null)}
        onRename={(id, name) => void handleRenameWorkspace(id, name)}
      />
      <SessionSidebar activeWorkspaceId={activeWorkspaceId ?? undefined} />
    </div>
  );
}
