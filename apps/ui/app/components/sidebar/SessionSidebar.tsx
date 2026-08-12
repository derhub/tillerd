import type { Session } from "@tillerd/client-bindings";

import { useMutation, useQuery } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { command, getQueryClient, query, reorder } from "@tillerd/client-bindings";
import React from "react";

import { DeleteDialog, type DeleteTarget } from "~/components/sidebar/DeleteDialog";
import {
  MovePickerDialog,
  StopSurfacesDialog,
  type MoveTarget,
  type StopSurfacesTarget,
} from "~/components/sidebar/EntityDialogs";
import { NewProjectButton } from "~/components/sidebar/NewProjectButton";
import { NewProjectDialog } from "~/components/sidebar/NewProjectDialog";
import {
  NewSessionTemplateDialog,
  type NewSessionTemplateTarget,
} from "~/components/sidebar/NewSessionTemplateDialog";
import { ProjectTree, type ProjectTreeHandlers } from "~/components/sidebar/ProjectTree";
import { SessionSearchDialog } from "~/components/sidebar/SessionSearchDialog";
import { UNFILED_ID, useSidebarData } from "~/components/sidebar/sidebar-data";
import { ScrollArea } from "~/components/ui/scroll-area";
import { ACTION, SESSION_SEARCH_ACTION_ID } from "~/lib/commands/ids";
import { type CommandArgs, useRegisterHandlers } from "~/lib/commands/registry";
import { SESSION_SEARCH_OPEN_EVENT } from "~/lib/commands/sessionSearch";
import { projectSettingsQuery } from "~/lib/data/settings";
import { templateListQuery } from "~/lib/data/templates";
import {
  librarySpecFor,
  resolveDefaultTemplate,
  type TemplateSelection,
} from "~/lib/newSessionTemplate";
import { notify } from "~/lib/notifications/notify";
import { mountSessionStatus } from "~/lib/sessionStatus";
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

const newSessionArgs = (projectId: string, templateId: string | null = null) => ({
  projectId,
  title: null,
  titleSource: "agent-title",
  templateId,
});

// Plain helper (not a hook/component) -- resolves a project's configured default
// template (ui-settings-editor spec: "Project settings -> Default template
// honored") before the plain new-session control creates a session. The
// no-async-in-component rule exempts standalone functions like this; components
// fire mutations via mutate(), never await/.then() themselves.
function resolveProjectDefault(
  projectId: string,
  onResolved: (selection: TemplateSelection | null) => void,
): void {
  getQueryClient()
    .fetchQuery({ ...projectSettingsQuery(projectId), staleTime: 0 })
    .then(
      (settings) => onResolved(resolveDefaultTemplate(settings)),
      () => onResolved(null),
    );
}

// Resolves a library template's spec on demand, freshly fetched (staleTime 0) so a
// template deleted or edited since the sidebar mounted is seen -- session_create only
// accepts a launch-template id, so the library entry must be materialized first. Null
// on a missing id (deleted template) or a failed fetch, letting the caller surface it.
// Standalone like resolveProjectDefault: keeps the async fetch out of the component.
function resolveLibrarySpec(
  templateId: string,
  onResolved: (spec: { specVersion: number; specJson: string } | null) => void,
): void {
  getQueryClient()
    .fetchQuery({ ...templateListQuery(), staleTime: 0 })
    .then(
      (templates) => onResolved(librarySpecFor(templates, templateId)),
      () => onResolved(null),
    );
}

// The row's display name, carried in a context-menu command's args (see
// EntityContextMenu) for confirmation/picker dialogs -- the row itself, not this
// component, knows the project/session name for an arbitrary entityId.
function labelArg(args: CommandArgs | undefined): string {
  return typeof args?.label === "string" ? args.label : "";
}

function stringArg(args: CommandArgs | undefined, key: string): string {
  const v = args?.[key];
  return typeof v === "string" ? v : "";
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

  // Move-to-workspace targets; a light non-suspense read (the switcher already
  // suspends on this list, so it is warm) used only to populate the picker.
  const { data: workspaces = [] } = useQuery(query("workspaceList"));

  const createProject = useMutation(command("projectCreate"));
  const createSession = useMutation(command("sessionCreate"));
  const createLaunchTemplate = useMutation(command("launchTemplateCreate"));
  const renameProject = useMutation(command("projectRename"));
  const renameSession = useMutation(command("sessionRename"));
  const archiveProject = useMutation(command("projectArchive"));
  const archiveSession = useMutation(command("sessionArchive"));
  const restoreProject = useMutation(command("projectRestore"));
  const restoreSession = useMutation(command("sessionRestore"));
  const deleteProject = useMutation(command("projectDelete"));
  const deleteSession = useMutation(command("sessionDelete"));
  const duplicateProject = useMutation(command("projectDuplicate"));
  const duplicateSession = useMutation(command("sessionDuplicate"));
  const pinProject = useMutation(command("projectPin"));
  const unpinProject = useMutation(command("projectUnpin"));
  const pinSession = useMutation(command("sessionPin"));
  const unpinSession = useMutation(command("sessionUnpin"));
  const moveProject = useMutation(command("projectMove"));
  const moveSession = useMutation(command("sessionMove"));
  const stopProjectSurfaces = useMutation(command("projectStopSurfaces"));
  const stopSessionSurfaces = useMutation(command("sessionStopSurfaces"));
  const reorderProjects = useMutation(reorder("projectReorder"));
  const reorderSessions = useMutation(reorder("sessionReorder"));

  const [detachedProjects, setDetachedProjects] = React.useState<Set<string>>(() => new Set());
  const [editingId, setEditingId] = React.useState<string | null>(null);
  const [deleteConfirm, setDeleteConfirm] = React.useState<DeleteTarget | null>(null);
  const [moveTarget, setMoveTarget] = React.useState<MoveTarget | null>(null);
  const [stopTarget, setStopTarget] = React.useState<StopSurfacesTarget | null>(null);
  const [templatePickerTarget, setTemplatePickerTarget] =
    React.useState<NewSessionTemplateTarget | null>(null);
  const [newProjectOpen, setNewProjectOpen] = React.useState(false);

  // One per-window subscription feeding the session-row status badges.
  React.useEffect(() => mountSessionStatus(), []);

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

  const handleCreateProject = React.useCallback(
    (name: string) => {
      if (!isDesktop) return;
      createProject.mutate(
        { name: name || null, workspaceId: activeWorkspaceId ?? null },
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
    },
    [isDesktop, navigate, activeWorkspaceId, createProject, createSession],
  );

  const handleNewProject = React.useCallback(() => {
    if (isDesktop) setNewProjectOpen(true);
  }, [isDesktop]);

  // The single point where a session actually gets created from a resolved
  // choice: empty, one of the project's launch templates directly, or a library
  // template materialized into a project launch template first (session_create
  // only accepts a launch-template id -- ui-template-manager design decisions).
  const createSessionFromSelection = React.useCallback(
    (projectId: string, selection: TemplateSelection) => {
      const onCreated = (sess: Session) => {
        setActiveProject(projectId);
        void navigate({ to: `/session/${sess.id}` } as never);
      };
      if (selection.kind === "empty") {
        createSession.mutate(newSessionArgs(projectId), { onSuccess: onCreated });
        return;
      }
      if (selection.kind === "launch") {
        createSession.mutate(newSessionArgs(projectId, selection.id), { onSuccess: onCreated });
        return;
      }
      resolveLibrarySpec(selection.id, (spec) => {
        // Missing id (the configured default points at a since-deleted library
        // template) or a failed library fetch: surface it via the notification
        // center instead of silently doing nothing (repo rule: no toasts, the
        // notification center is the sole client feedback channel).
        if (!spec) {
          notify("new-session", "error", "That session template is no longer available.");
          return;
        }
        createLaunchTemplate.mutate(
          { projectId, specVersion: spec.specVersion, specJson: spec.specJson },
          {
            onSuccess: (tmpl) =>
              createSession.mutate(newSessionArgs(projectId, tmpl.id), { onSuccess: onCreated }),
          },
        );
      });
    },
    [navigate, createSession, createLaunchTemplate],
  );

  // The plain new-session control (the "+" row action, `ACTION.sessionNew`):
  // instantiates the project's configured default template silently, or an
  // empty session when none is set (ui-settings-editor spec: "Default template
  // honored"). It never opens the picker -- that is a separate, explicit row
  // action (`ACTION.projectNewSessionFromTemplate`).
  const handleNewSession = React.useCallback(
    (projectId: string) => {
      if (!isDesktop) return;
      resolveProjectDefault(projectId, (selection) => {
        createSessionFromSelection(projectId, selection ?? { kind: "empty" });
      });
    },
    [isDesktop, createSessionFromSelection],
  );

  const navHomeIfActiveSession = React.useCallback(
    (sessId: string) => {
      if (window.location.pathname === `/session/${sessId}`) void navigate({ to: "/" } as never);
    },
    [navigate],
  );

  const handleArchiveSession = React.useCallback(
    (sessId: string) => {
      if (!isDesktop) return;
      archiveSession.mutate({ id: sessId }, { onSuccess: () => navHomeIfActiveSession(sessId) });
    },
    [isDesktop, archiveSession, navHomeIfActiveSession],
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
      if (kind === "session") navHomeIfActiveSession(id);
      else void navigate({ to: "/" } as never);
    };
    if (kind === "project") deleteProject.mutate({ id }, { onSuccess });
    else if (kind === "session") deleteSession.mutate({ id }, { onSuccess });
  }, [isDesktop, deleteConfirm, navigate, deleteProject, deleteSession, navHomeIfActiveSession]);

  const handlePickMove = React.useCallback(
    (targetId: string) => {
      if (!moveTarget) return;
      if (moveTarget.kind === "project") {
        moveProject.mutate({ id: moveTarget.id, workspaceId: targetId });
      } else {
        moveSession.mutate({ id: moveTarget.id, targetProjectId: targetId });
      }
      setMoveTarget(null);
    },
    [moveTarget, moveProject, moveSession],
  );

  const handleConfirmStop = React.useCallback(() => {
    if (!stopTarget) return;
    if (stopTarget.kind === "project") stopProjectSurfaces.mutate({ id: stopTarget.id });
    else if (stopTarget.kind === "session") stopSessionSurfaces.mutate({ id: stopTarget.id });
    setStopTarget(null);
  }, [stopTarget, stopProjectSurfaces, stopSessionSurfaces]);

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

  // Move-to-project targets: the workspace's named projects plus Unfiled, minus the
  // session's current project. Computed from the already-fetched sidebar list.
  const projectMoveTargets = React.useCallback(
    (excludeProjectId: string) => {
      const named = projects
        .filter((p) => p.id !== UNFILED_ID && p.id !== excludeProjectId && p.status !== "archived")
        .map((p) => ({ id: p.id, name: p.name }));
      if (excludeProjectId !== UNFILED_ID) named.push({ id: UNFILED_ID, name: "Unfiled" });
      return named;
    },
    [projects],
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
      // right-clicked row's entityId (and, where needed, its label / parent id) as
      // args; this is the one place each handler is registered, per the registry's
      // one-handler-per-id model.
      [ACTION.projectOpenNewWindowRow]: (args?: CommandArgs) => {
        if (args?.entityId) handleOpenInNewWindow(args.entityId);
      },
      [ACTION.projectNewSessionFromTemplate]: (args?: CommandArgs) => {
        if (args?.entityId)
          setTemplatePickerTarget({ projectId: args.entityId, projectName: labelArg(args) });
      },
      [ACTION.projectRename]: (args?: CommandArgs) => {
        if (args?.entityId) setEditingId(args.entityId);
      },
      [ACTION.projectDuplicate]: (args?: CommandArgs) => {
        if (args?.entityId) duplicateProject.mutate({ sourceId: args.entityId, name: null });
      },
      [ACTION.projectPin]: (args?: CommandArgs) => {
        if (args?.entityId) pinProject.mutate({ id: args.entityId });
      },
      [ACTION.projectUnpin]: (args?: CommandArgs) => {
        if (args?.entityId) unpinProject.mutate({ id: args.entityId });
      },
      [ACTION.projectMove]: (args?: CommandArgs) => {
        if (!args?.entityId) return;
        setMoveTarget({
          id: args.entityId,
          name: labelArg(args),
          kind: "project",
          targets: workspaces
            .filter((w) => w.id !== stringArg(args, "workspaceId") && w.status !== "archived")
            .map((w) => ({ id: w.id, name: w.name })),
        });
      },
      [ACTION.projectStopSurfaces]: (args?: CommandArgs) => {
        if (args?.entityId)
          setStopTarget({ id: args.entityId, name: labelArg(args), kind: "project" });
      },
      [ACTION.projectArchive]: (args?: CommandArgs) => {
        if (!args?.entityId) return;
        const id = args.entityId;
        archiveProject.mutate({ id }, { onSuccess: () => void navigate({ to: "/" } as never) });
      },
      [ACTION.projectDelete]: (args?: CommandArgs) => {
        if (!args?.entityId) return;
        setDeleteConfirm({ id: args.entityId, name: labelArg(args), kind: "project" });
      },
      [ACTION.sessionRename]: (args?: CommandArgs) => {
        if (args?.entityId) setEditingId(args.entityId);
      },
      [ACTION.sessionDuplicate]: (args?: CommandArgs) => {
        if (args?.entityId) duplicateSession.mutate({ id: args.entityId });
      },
      [ACTION.sessionPin]: (args?: CommandArgs) => {
        if (args?.entityId) pinSession.mutate({ id: args.entityId });
      },
      [ACTION.sessionUnpin]: (args?: CommandArgs) => {
        if (args?.entityId) unpinSession.mutate({ id: args.entityId });
      },
      [ACTION.sessionMove]: (args?: CommandArgs) => {
        if (!args?.entityId) return;
        setMoveTarget({
          id: args.entityId,
          name: labelArg(args),
          kind: "session",
          targets: projectMoveTargets(stringArg(args, "projectId")),
        });
      },
      [ACTION.sessionStopSurfaces]: (args?: CommandArgs) => {
        if (args?.entityId)
          setStopTarget({ id: args.entityId, name: labelArg(args), kind: "session" });
      },
      [ACTION.sessionArchive]: (args?: CommandArgs) => {
        if (args?.entityId) handleArchiveSession(args.entityId);
      },
      [ACTION.sessionDelete]: (args?: CommandArgs) => {
        if (!args?.entityId) return;
        setDeleteConfirm({ id: args.entityId, name: labelArg(args), kind: "session" });
      },
    };
  }, [
    activeProjectId,
    projects,
    workspaces,
    navigate,
    handleNewProject,
    handleNewSession,
    handleOpenInNewWindow,
    handleArchiveSession,
    duplicateProject,
    duplicateSession,
    pinProject,
    unpinProject,
    pinSession,
    unpinSession,
    archiveProject,
    projectMoveTargets,
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
    onArchiveSession: (id) => handleArchiveSession(id),
    onRestoreProject: (id) => restoreProject.mutate({ id }),
    onRestoreSession: (id) => restoreSession.mutate({ id }),
    onRequestDelete: (target) => setDeleteConfirm(target),
    onFocusDetached: handleReattachProject,
  };

  return (
    <div className="flex flex-col h-full">
      <DeleteDialog
        target={deleteConfirm}
        onCancel={() => setDeleteConfirm(null)}
        onConfirm={() => handleConfirmDelete()}
      />
      <NewProjectDialog
        open={newProjectOpen}
        onOpenChange={setNewProjectOpen}
        onCreate={handleCreateProject}
      />
      <MovePickerDialog
        target={moveTarget}
        onCancel={() => setMoveTarget(null)}
        onPick={handlePickMove}
      />
      <StopSurfacesDialog
        target={stopTarget}
        onCancel={() => setStopTarget(null)}
        onConfirm={handleConfirmStop}
      />
      <NewSessionTemplateDialog
        target={templatePickerTarget}
        onCancel={() => setTemplatePickerTarget(null)}
        onSelect={(selection) => {
          if (!templatePickerTarget) return;
          createSessionFromSelection(templatePickerTarget.projectId, selection);
          setTemplatePickerTarget(null);
        }}
      />

      <SessionSearchDialog />

      <div className="px-3 pt-2 pb-1 shrink-0 flex items-center gap-1">
        {isDesktop && <NewProjectButton onClick={() => handleNewProject()} />}
      </div>

      <ScrollArea className="flex-1 min-h-0">
        {projects.length === 0 ? (
          <div
            data-testid="sidebar-empty"
            className="flex flex-col items-center gap-1 px-4 py-8 text-center"
          >
            <p className="text-[0.833rem] text-muted-foreground/60">No projects yet</p>
            {isDesktop && (
              <button
                type="button"
                onClick={() => handleNewProject()}
                className="text-[0.833rem] text-muted-foreground hover:text-foreground underline underline-offset-2"
              >
                Create a project
              </button>
            )}
          </div>
        ) : (
          <ProjectTree projects={projects} handlers={treeHandlers} />
        )}
      </ScrollArea>
    </div>
  );
}
