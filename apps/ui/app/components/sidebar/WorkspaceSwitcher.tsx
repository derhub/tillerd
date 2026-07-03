import { useQuery, useSuspenseQuery } from "@tanstack/react-query";
import { useMutation } from "@tanstack/react-query";
import {
  command,
  query,
  type Workspace,
  type WorkspaceActivityView,
} from "@tillerd/client-bindings";
import { FolderPlus, ArrowUpRight } from "lucide-react";
import React from "react";

import { InlineRenameInput } from "~/components/sidebar/InlineRenameInput";
import { SessionSidebar } from "~/components/sidebar/SessionSidebar";
import { DEFAULT_WORKSPACE_ID } from "~/components/sidebar/sidebar-data";
import { useActiveWorkspace, setActiveWorkspace } from "~/lib/store";
import { subscribe } from "~/lib/subscribe";
import { useDesktopHost } from "~/lib/useDesktopHost";
import { cn } from "~/lib/utils";
import {
  closeWindow,
  focusSelf,
  onReattachWorkspace,
  openWindow,
  workspaceLabel,
  workspaceQuery,
} from "~/lib/windows";

export interface WorkspaceSwitcherProps {
  workspaces: Workspace[];
  activity?: ReadonlyMap<string, WorkspaceActivityView>;
  activeId: string | null;
  detachedIds: Set<string>;
  isDesktop: boolean;
  onSelect: (id: string) => void;
  onNewWorkspace: () => void;
  onDetach: (id: string) => void;
  onReattach: (id: string) => void;
  editingId: string | null;
  onStartEdit: (id: string) => void;
  onCancelEdit: () => void;
  onRename: (id: string, name: string) => void;
}

// Minimal activity signal (full badge styling lands with the 0.0.20 visual pass):
// a dot per workspace with live/failed surfaces, colored by the worst state.
function ActivityDot({ activity }: { activity?: WorkspaceActivityView }) {
  if (!activity || (activity.running === 0 && activity.failed === 0)) return null;
  const failed = activity.failed > 0;
  return (
    <span
      data-testid="workspace-activity"
      data-running={activity.running}
      data-failed={activity.failed}
      title={`${activity.running} running, ${activity.failed} failed`}
      className={cn(
        "size-1.5 rounded-full shrink-0",
        failed ? "bg-red-500" : "bg-emerald-500",
      )}
    />
  );
}

export function WorkspaceSwitcherList({
  workspaces,
  activity,
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
          <ActivityDot activity={activity?.get(ws.id)} />
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

export function WorkspaceSwitcher({ initialWorkspaceId }: { initialWorkspaceId?: string } = {}) {
  const isDesktop = useDesktopHost().status === "ready";
  const storedActiveWorkspaceId = useActiveWorkspace();
  const [detachedWorkspaces, setDetachedWorkspaces] = React.useState<Set<string>>(() => new Set());
  const [editingId, setEditingId] = React.useState<string | null>(null);

  const createWorkspace = useMutation(command("workspaceCreate"));
  const renameWorkspace = useMutation(command("workspaceRename"));

  const { data: workspaces } = useSuspenseQuery(query("workspaceList"));
  // Enhancement read: the switcher renders without it (no suspense), the dot
  // appears when the rollup lands and refreshes on the surface-status push.
  const { data: activityRows } = useQuery(query("workspaceActivity"));
  const activity = React.useMemo(
    () => new Map((activityRows ?? []).map((a) => [a.workspaceId, a])),
    [activityRows],
  );

  // Lifecycle resolution (ADR-0044): a pointer to an archived or deleted workspace
  // resolves to the Default workspace — never an error or an empty shell.
  const pointerTarget = workspaces.find((w) => w.id === storedActiveWorkspaceId);
  const pointerStale =
    storedActiveWorkspaceId != null && (!pointerTarget || pointerTarget.status === "archived");
  const activeWorkspaceId = pointerStale
    ? DEFAULT_WORKSPACE_ID
    : (pointerTarget?.id ?? null);

  // Rewrite the stale pointer once so it does not re-resolve every start.
  React.useEffect(() => {
    if (pointerStale) setActiveWorkspace(DEFAULT_WORKSPACE_ID);
  }, [pointerStale]);

  React.useEffect(() => {
    if (initialWorkspaceId) setActiveWorkspace(initialWorkspaceId);
  }, [initialWorkspaceId]);

  React.useEffect(
    () =>
      subscribe(
        onReattachWorkspace(({ workspaceId }) => {
          setDetachedWorkspaces((prev) => {
            if (!prev.has(workspaceId)) return prev;
            const next = new Set(prev);
            next.delete(workspaceId);
            return next;
          });
          void focusSelf();
        }),
      ),
    [],
  );

  const handleRenameWorkspace = React.useCallback(
    (id: string, newName: string) => {
      if (!isDesktop) return;
      renameWorkspace.mutate(
        { id, name: newName.trim() || "New workspace" },
        { onSuccess: () => setEditingId(null) },
      );
    },
    [isDesktop, renameWorkspace],
  );

  // Tauri webview: window.prompt returns null, so naming must happen in-app via inline rename.
  const handleNewWorkspace = React.useCallback(() => {
    if (!isDesktop) return;
    createWorkspace.mutate(
      { name: "New workspace" },
      {
        onSuccess: (ws) => {
          setActiveWorkspace(ws.id);
          setEditingId(ws.id);
        },
      },
    );
  }, [isDesktop, createWorkspace]);

  const handleDetach = React.useCallback((workspaceId: string) => {
    void openWindow(workspaceLabel(workspaceId), workspaceQuery(workspaceId));
    setDetachedWorkspaces((prev) => new Set(prev).add(workspaceId));
  }, []);

  const handleReattach = React.useCallback((workspaceId: string) => {
    void closeWindow(workspaceLabel(workspaceId));
    // Clear immediately; the child re-attach event may not fire if the child closes before arming.
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
        activity={activity}
        activeId={activeWorkspaceId}
        detachedIds={detachedWorkspaces}
        isDesktop={isDesktop}
        onSelect={setActiveWorkspace}
        onNewWorkspace={() => handleNewWorkspace()}
        onDetach={handleDetach}
        onReattach={handleReattach}
        editingId={editingId}
        onStartEdit={setEditingId}
        onCancelEdit={() => setEditingId(null)}
        onRename={(id, name) => handleRenameWorkspace(id, name)}
      />
      <SessionSidebar activeWorkspaceId={activeWorkspaceId ?? undefined} />
    </div>
  );
}
