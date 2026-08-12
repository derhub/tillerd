import { useQuery, useSuspenseQuery } from "@tanstack/react-query";
import { useMutation } from "@tanstack/react-query";
import {
  command,
  query,
  type Workspace,
  type WorkspaceActivityView,
} from "@tillerd/client-bindings";
import { FolderPlus, ArrowUpRight, Pin } from "lucide-react";
import React from "react";

import { EntityContextMenu } from "~/components/shell/EntityContextMenu";
import { ArchivedRow, ArchivedSection } from "~/components/sidebar/ArchivedSection";
import { DeleteDialog, type DeleteTarget } from "~/components/sidebar/DeleteDialog";
import { StopSurfacesDialog, type StopSurfacesTarget } from "~/components/sidebar/EntityDialogs";
import { InlineRenameInput } from "~/components/sidebar/InlineRenameInput";
import { SessionSidebar } from "~/components/sidebar/SessionSidebar";
import { DEFAULT_WORKSPACE_ID } from "~/components/sidebar/sidebar-data";
import { Tooltip, TooltipContent, TooltipTrigger } from "~/components/ui/tooltip";
import { ACTION } from "~/lib/commands/ids";
import { type CommandArgs, useRegisterHandlers } from "~/lib/commands/registry";
import { can } from "~/lib/stateModel";
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
  onRestore?: (id: string) => void;
  onRequestDelete?: (target: DeleteTarget) => void;
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
        failed ? "bg-red-700 dark:bg-red-400" : "bg-emerald-700 dark:bg-emerald-400",
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
  onRestore,
  onRequestDelete,
}: WorkspaceSwitcherProps) {
  const active = workspaces.filter((ws) => ws.status !== "archived");
  const archived = workspaces.filter((ws) => ws.status === "archived");

  return (
    <div
      data-testid="workspace-switcher"
      className="flex flex-col gap-0.5 px-3 py-2 border-b border-border/40 shrink-0"
    >
      {active.map((ws) => (
        <div key={ws.id} className="flex items-center gap-1">
          {editingId === ws.id ? (
            <InlineRenameInput
              initialValue={ws.name}
              onConfirm={(name) => onRename(ws.id, name)}
              onCancel={onCancelEdit}
            />
          ) : (
            <EntityContextMenu
              entityId={ws.id}
              entityKind="workspace"
              args={{ label: ws.name }}
              guards={{
                "menu.canArchive": can("workspace", "archive", ws),
                "menu.canDelete": can("workspace", "discard", ws),
                "menu.pinned": ws.pinned,
              }}
              disabled={!isDesktop}
              className="flex-1 min-w-0"
            >
              <button
                type="button"
                data-testid="workspace-item"
                data-workspace-id={ws.id}
                onClick={() => onSelect(ws.id)}
                aria-current={ws.id === activeId ? "true" : undefined}
                onDoubleClick={() => onStartEdit(ws.id)}
                className={cn(
                  "w-full text-left text-[0.75rem] truncate px-2 py-0.5 rounded-sm transition-colors duration-[var(--motion-fast)] ease-standard",
                  ws.id === activeId
                    ? "font-medium bg-muted text-foreground"
                    : "text-muted-foreground hover:text-foreground hover:bg-muted",
                )}
              >
                {ws.name}
              </button>
            </EntityContextMenu>
          )}
          {ws.pinned && (
            <Pin
              strokeWidth={2}
              aria-hidden
              data-testid="workspace-pinned-indicator"
              className="shrink-0 text-muted-foreground/40 size-[var(--icon-sm)]"
            />
          )}
          <ActivityDot activity={activity?.get(ws.id)} />
          {detachedIds.has(ws.id) ? (
            <Tooltip>
              <TooltipTrigger
                type="button"
                onClick={() => onReattach(ws.id)}
                aria-label={`Re-attach ${ws.name}`}
                data-testid="workspace-detached-indicator"
                data-workspace-id={ws.id}
                className={cn(
                  "flex items-center p-0.5 rounded-sm transition-colors duration-[var(--motion-fast)] ease-standard",
                  "text-amber-700 hover:text-amber-800 dark:text-amber-400 dark:hover:text-amber-300 hover:bg-muted focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
                )}
              >
                <ArrowUpRight strokeWidth={2} className="size-[var(--icon-sm)]" />
              </TooltipTrigger>
              <TooltipContent>Re-attach {ws.name}</TooltipContent>
            </Tooltip>
          ) : (
            isDesktop && (
              <Tooltip>
                <TooltipTrigger
                  type="button"
                  onClick={() => onDetach(ws.id)}
                  aria-label={`Open ${ws.name} in a new window`}
                  data-testid="workspace-detach"
                  data-workspace-id={ws.id}
                  className={cn(
                    "flex items-center p-0.5 rounded-sm transition-colors duration-[var(--motion-fast)] ease-standard",
                    "text-muted-foreground hover:text-foreground hover:bg-muted focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
                  )}
                >
                  <ArrowUpRight strokeWidth={2} className="size-[var(--icon-sm)]" />
                </TooltipTrigger>
                <TooltipContent>Open {ws.name} in a new window</TooltipContent>
              </Tooltip>
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
          <FolderPlus strokeWidth={2} className="size-[var(--icon-sm)]" />
          <span>New workspace</span>
        </button>
      )}
      <ArchivedSection count={archived.length}>
        {archived.map((ws) => (
          <ArchivedRow
            key={ws.id}
            name={ws.name}
            onRestore={() => onRestore?.(ws.id)}
            onDelete={() => onRequestDelete?.({ id: ws.id, name: ws.name, kind: "workspace" })}
          />
        ))}
      </ArchivedSection>
    </div>
  );
}

export function WorkspaceSwitcher({ initialWorkspaceId }: { initialWorkspaceId?: string } = {}) {
  const isDesktop = useDesktopHost().status === "ready";
  const storedActiveWorkspaceId = useActiveWorkspace();
  const [detachedWorkspaces, setDetachedWorkspaces] = React.useState<Set<string>>(() => new Set());
  const [editingId, setEditingId] = React.useState<string | null>(null);
  const [deleteConfirm, setDeleteConfirm] = React.useState<DeleteTarget | null>(null);
  const [stopTarget, setStopTarget] = React.useState<StopSurfacesTarget | null>(null);

  const createWorkspace = useMutation(command("workspaceCreate"));
  const renameWorkspace = useMutation(command("workspaceRename"));
  const pinWorkspace = useMutation(command("workspacePin"));
  const unpinWorkspace = useMutation(command("workspaceUnpin"));
  const archiveWorkspace = useMutation(command("workspaceArchive"));
  const restoreWorkspace = useMutation(command("workspaceRestore"));
  const deleteWorkspace = useMutation(command("workspaceDelete"));
  const stopWorkspaceSurfaces = useMutation(command("workspaceStopSurfaces"));

  const { data: workspaces, isFetching: isFetchingWorkspaces } = useSuspenseQuery(
    query("workspaceList"),
  );
  // Enhancement read: the switcher renders without it (no suspense), the dot
  // appears when the rollup lands and refreshes on the surface-status push.
  const { data: activityRows } = useQuery(query("workspaceActivity"));
  const activity = React.useMemo(
    () => new Map((activityRows ?? []).map((a) => [a.workspaceId, a])),
    [activityRows],
  );

  // A window opened with an explicit workspace intent (detached workspace
  // window) scopes to it window-locally; writing the shared global pointer here
  // would live-rescope every other window through the settings broadcast.
  const scopedId = initialWorkspaceId ?? storedActiveWorkspaceId;

  // Lifecycle resolution: a pointer to an archived or deleted workspace resolves
  // to the Default workspace -- never an error or an empty shell. Staleness is
  // judged only against a settled list: right after a create, the pointer names a
  // workspace the cached list does not carry yet, and treating that as stale
  // would clobber the user's fresh selection back to Default.
  const pointerTarget = workspaces.find((w) => w.id === scopedId);
  const pointerStale =
    !isFetchingWorkspaces &&
    scopedId != null &&
    (!pointerTarget || pointerTarget.status === "archived");
  const activeWorkspaceId = pointerStale ? DEFAULT_WORKSPACE_ID : scopedId;

  // Rewrite the pointer only for a target the list KNOWS is archived. An absent
  // id is not proof of deletion (the settled list can be a stale restored
  // snapshot missing a young workspace, or a failed cold-start refetch): render
  // the Default scope but keep the pointer, which self-heals when the list
  // catches up.
  React.useEffect(() => {
    if (pointerStale && !initialWorkspaceId && pointerTarget?.status === "archived") {
      setActiveWorkspace(DEFAULT_WORKSPACE_ID);
    }
  }, [pointerStale, pointerTarget?.status, initialWorkspaceId]);

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

  const handleConfirmDelete = React.useCallback(() => {
    if (!deleteConfirm) return;
    const { id } = deleteConfirm;
    deleteWorkspace.mutate({ id }, { onSuccess: () => setDeleteConfirm(null) });
  }, [deleteConfirm, deleteWorkspace]);

  const handleConfirmStop = React.useCallback(() => {
    if (!stopTarget) return;
    stopWorkspaceSurfaces.mutate({ id: stopTarget.id });
    setStopTarget(null);
  }, [stopTarget, stopWorkspaceSurfaces]);

  // Row-scoped workspace context-menu handlers (one registration per id).
  const workspaceHandlers = React.useMemo(
    () => ({
      [ACTION.workspaceRename]: (args?: CommandArgs) => {
        if (args?.entityId) setEditingId(args.entityId);
      },
      [ACTION.workspacePin]: (args?: CommandArgs) => {
        if (args?.entityId) pinWorkspace.mutate({ id: args.entityId });
      },
      [ACTION.workspaceUnpin]: (args?: CommandArgs) => {
        if (args?.entityId) unpinWorkspace.mutate({ id: args.entityId });
      },
      [ACTION.workspaceStopSurfaces]: (args?: CommandArgs) => {
        if (args?.entityId)
          setStopTarget({
            id: args.entityId,
            name: typeof args.label === "string" ? args.label : "",
            kind: "workspace",
          });
      },
      [ACTION.workspaceArchive]: (args?: CommandArgs) => {
        if (args?.entityId) archiveWorkspace.mutate({ id: args.entityId });
      },
      [ACTION.workspaceDelete]: (args?: CommandArgs) => {
        if (!args?.entityId) return;
        setDeleteConfirm({
          id: args.entityId,
          name: typeof args.label === "string" ? args.label : "",
          kind: "workspace",
        });
      },
    }),
    [pinWorkspace, unpinWorkspace, archiveWorkspace],
  );
  useRegisterHandlers(workspaceHandlers);

  return (
    <div className="flex flex-col h-full">
      <DeleteDialog
        target={deleteConfirm}
        onCancel={() => setDeleteConfirm(null)}
        onConfirm={handleConfirmDelete}
      />
      <StopSurfacesDialog
        target={stopTarget}
        onCancel={() => setStopTarget(null)}
        onConfirm={handleConfirmStop}
      />
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
        onRestore={(id) => restoreWorkspace.mutate({ id })}
        onRequestDelete={(target) => setDeleteConfirm(target)}
      />
      <SessionSidebar activeWorkspaceId={activeWorkspaceId ?? undefined} />
    </div>
  );
}
