import { useMutation } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { command, reorder } from "@tillerd/client-bindings";
import React from "react";

import { DeleteDialog, type DeleteTarget } from "~/components/sidebar/DeleteDialog";
import { NewProjectButton } from "~/components/sidebar/NewProjectButton";
import { ProjectTree, type ProjectTreeHandlers } from "~/components/sidebar/ProjectTree";
import { SessionSearchDialog } from "~/components/sidebar/SessionSearchDialog";
import { useSidebarData } from "~/components/sidebar/sidebar-data";
import { ScrollArea } from "~/components/ui/scroll-area";
import { ACTION, SESSION_SEARCH_ACTION_ID } from "~/lib/commands/ids";
import { type CommandArgs, useRegisterHandlers } from "~/lib/commands/registry";
import { SESSION_SEARCH_OPEN_EVENT } from "~/lib/commands/sessionSearch";
import { useActiveProject, setActiveProject } from "~/lib/store";
import { subscribe } from "~/lib/subscribe";
import { useDesktopHost } from "~/lib/useDesktopHost";
import {
  closeWindow,
  focusSelf,
  onReattachProject,
  openWindow,
  projectLabel,
  projectQuery,
} from "~/lib/windows";

const newSessionArgs = (projectId: string) => ({
  projectId,
  title: null,
  titleSource: "agent-title",
  templateId: null,
});

// The row's display name, carried in a context-menu command's args (see
// EntityContextMenu) for the delete-confirmation dialog -- the row itself, not
// this component, knows the project/session name for an arbitrary entityId.
function labelArg(args: CommandArgs | undefined): string {
  return typeof args?.label === "string" ? args.label : "";
}

export function SessionSidebar({
  activeWorkspaceId,
  activeProjectId: propActiveProjectId,
}: { activeWorkspaceId?: string; activeProjectId?: string } = {}) {
  const isDesktop = useDesktopHost().status === "ready";
  const navigate = useNavigate();

  const storeActiveProjectId = useActiveProject();
  const activeProjectId = propActiveProjectId ?? storeActiveProjectId;
  const { projects } = useSidebarData(activeWorkspaceId, propActiveProjectId);

  if (propActiveProjectId && storeActiveProjectId !== propActiveProjectId) {
    setActiveProject(propActiveProjectId);
  }

  const createProject = useMutation(command("projectCreate"));
  const createSession = useMutation(command("sessionCreate"));
  const renameProject = useMutation(command("projectRename"));
  const renameSession = useMutation(command("sessionRename"));
  const archiveSession = useMutation(command("sessionArchive"));
  const deleteProject = useMutation(command("projectDelete"));
  const deleteSession = useMutation(command("sessionDelete"));
  const reorderProjects = useMutation(reorder("projectReorder"));
  const reorderSessions = useMutation(reorder("sessionReorder"));

  const [detachedProjects, setDetachedProjects] = React.useState<Set<string>>(() => new Set());
  const [editingId, setEditingId] = React.useState<string | null>(null);
  const [deleteConfirm, setDeleteConfirm] = React.useState<DeleteTarget | null>(null);

  const handleOpenInNewWindow = React.useCallback((projectId: string) => {
    void openWindow(projectLabel(projectId), projectQuery(projectId, null));
    setDetachedProjects((prev) => new Set(prev).add(projectId));
  }, []);

  const clearDetached = React.useCallback((projectId: string) => {
    setDetachedProjects((prev) => {
      if (!prev.has(projectId)) return prev;
      const next = new Set(prev);
      next.delete(projectId);
      return next;
    });
  }, []);

  const handleReattachProject = React.useCallback(
    (projectId: string) => {
      void closeWindow(projectLabel(projectId));
      clearDetached(projectId);
    },
    [clearDetached],
  );

  React.useEffect(
    () =>
      subscribe(
        onReattachProject(({ projectId }) => {
          clearDetached(projectId);
          void focusSelf();
        }),
      ),
    [clearDetached],
  );

  const handleNewProject = React.useCallback(() => {
    if (!isDesktop) return;
    const name = window.prompt("Project name (leave blank for a blank project):") ?? "";
    createProject.mutate(
      { name: name.trim() || null, workspaceId: activeWorkspaceId ?? null },
      {
        onSuccess: (proj) => {
          void navigate({ to: "/" } as never);
          createSession.mutate(newSessionArgs(proj.id), {
            onSuccess: (sess) => {
              setActiveProject(proj.id);
              void navigate({ to: `/session/${sess.id}` } as never);
            },
          });
        },
      },
    );
  }, [isDesktop, navigate, activeWorkspaceId, createProject, createSession]);

  const handleNewSession = React.useCallback(
    (projectId: string) => {
      if (!isDesktop) return;
      createSession.mutate(newSessionArgs(projectId), {
        onSuccess: (sess) => {
          setActiveProject(projectId);
          void navigate({ to: `/session/${sess.id}` } as never);
        },
      });
    },
    [isDesktop, navigate, createSession],
  );

  const handleArchiveSession = React.useCallback(
    (sessId: string, currentPath: string) => {
      if (!isDesktop) return;
      archiveSession.mutate(
        { id: sessId },
        {
          onSuccess: () => {
            if (currentPath === `/session/${sessId}`) void navigate({ to: "/" } as never);
          },
        },
      );
    },
    [isDesktop, navigate, archiveSession],
  );

  const handleRenameProject = React.useCallback(
    (projectId: string, newName: string) => {
      if (!isDesktop) return;
      renameProject.mutate(
        { id: projectId, name: newName },
        { onSuccess: () => setEditingId(null) },
      );
    },
    [isDesktop, renameProject],
  );

  const handleRenameSession = React.useCallback(
    (sessId: string, newName: string) => {
      if (!isDesktop) return;
      renameSession.mutate({ id: sessId, title: newName }, { onSuccess: () => setEditingId(null) });
    },
    [isDesktop, renameSession],
  );

  const handleConfirmDelete = React.useCallback(() => {
    if (!isDesktop || !deleteConfirm) return;
    const { id, kind } = deleteConfirm;
    const onSuccess = () => {
      setDeleteConfirm(null);
      if (kind === "session" && window.location.pathname === `/session/${id}`) {
        void navigate({ to: "/" } as never);
      } else if (kind === "project") {
        void navigate({ to: "/" } as never);
      }
    };
    if (kind === "project") deleteProject.mutate({ id }, { onSuccess });
    else deleteSession.mutate({ id }, { onSuccess });
  }, [isDesktop, deleteConfirm, navigate, deleteProject, deleteSession]);

  const handleReorderProjects = React.useCallback(
    (orderedIds: string[]) => {
      if (isDesktop) reorderProjects.mutate(orderedIds);
    },
    [isDesktop, reorderProjects],
  );

  const handleReorderSessions = React.useCallback(
    (orderedIds: string[]) => {
      if (isDesktop) reorderSessions.mutate(orderedIds);
    },
    [isDesktop, reorderSessions],
  );

  const sidebarHandlers = React.useMemo(() => {
    const targetProjectId = (): string => activeProjectId ?? projects[0]?.id ?? "";
    return {
      [ACTION.projectNew]: () => handleNewProject(),
      [ACTION.sessionNew]: () => handleNewSession(targetProjectId()),
      [SESSION_SEARCH_ACTION_ID]: () =>
        window.dispatchEvent(new CustomEvent(SESSION_SEARCH_OPEN_EVENT)),
      [ACTION.projectOpenNewWindow]: () => handleOpenInNewWindow(targetProjectId()),
      // Row-scoped context-menu actions -- EntityContextMenu passes the
      // right-clicked row's entityId (and, where needed, its label) as args;
      // this is the one place each handler is registered, per the registry's
      // one-handler-per-id model.
      [ACTION.projectOpenNewWindowRow]: (args?: CommandArgs) => {
        if (args?.entityId) handleOpenInNewWindow(args.entityId);
      },
      [ACTION.projectRename]: (args?: CommandArgs) => {
        if (args?.entityId) setEditingId(args.entityId);
      },
      [ACTION.projectDelete]: (args?: CommandArgs) => {
        if (!args?.entityId) return;
        setDeleteConfirm({ id: args.entityId, name: labelArg(args), kind: "project" });
      },
      [ACTION.sessionRename]: (args?: CommandArgs) => {
        if (args?.entityId) setEditingId(args.entityId);
      },
      [ACTION.sessionArchive]: (args?: CommandArgs) => {
        if (args?.entityId) handleArchiveSession(args.entityId, window.location.pathname);
      },
      [ACTION.sessionDelete]: (args?: CommandArgs) => {
        if (!args?.entityId) return;
        setDeleteConfirm({ id: args.entityId, name: labelArg(args), kind: "session" });
      },
    };
  }, [
    activeProjectId,
    projects,
    handleNewProject,
    handleNewSession,
    handleOpenInNewWindow,
    handleArchiveSession,
  ]);
  useRegisterHandlers(sidebarHandlers);

  const treeHandlers: ProjectTreeHandlers = {
    isDesktop,
    editingId,
    isDetached: (id) => detachedProjects.has(id),
    onStartEdit: setEditingId,
    onStartEditSession: setEditingId,
    onCancelEdit: () => setEditingId(null),
    onRenameProject: (id, newName) => handleRenameProject(id, newName),
    onRenameSession: handleRenameSession,
    onReorderProjects: handleReorderProjects,
    onReorderSessions: handleReorderSessions,
    onNewSession: (id) => handleNewSession(id),
    onArchiveSession: handleArchiveSession,
    onFocusDetached: handleReattachProject,
  };

  return (
    <div className="flex flex-col h-full">
      <DeleteDialog
        target={deleteConfirm}
        onCancel={() => setDeleteConfirm(null)}
        onConfirm={() => handleConfirmDelete()}
      />

      <SessionSearchDialog />

      <div className="px-3 pt-2 pb-1 shrink-0 flex items-center gap-1">
        {isDesktop && <NewProjectButton onClick={() => handleNewProject()} />}
      </div>

      <ScrollArea className="flex-1 min-h-0">
        {projects.length === 0 ? (
          <p className="px-3 py-3 text-[0.917rem] text-muted-foreground/50 italic">No projects</p>
        ) : (
          <ProjectTree projects={projects} handlers={treeHandlers} />
        )}
      </ScrollArea>
    </div>
  );
}
