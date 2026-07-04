import type { Project } from "@tillerd/client-bindings";

import { ProjectRow } from "~/components/sidebar/ProjectRow";
import { DEFAULT_WORKSPACE_ID, UNFILED_ID } from "~/components/sidebar/sidebar-data";

export interface ProjectTreeHandlers {
  isDesktop: boolean;
  editingId: string | null;
  isDetached: (projectId: string) => boolean;
  onStartEdit: (id: string) => void;
  onStartEditSession: (sessionId: string) => void;
  onCancelEdit: () => void;
  onRenameProject: (projectId: string, newName: string) => void;
  onRenameSession: (sessionId: string, newName: string) => void;
  onReorderProjects: (orderedIds: string[]) => void;
  onReorderSessions: (orderedIds: string[]) => void;
  onNewSession: (projectId: string) => void;
  onArchiveSession: (id: string, currentPath: string) => void;
  onFocusDetached: (projectId: string) => void;
}

// Unfiled always renders: emptiness is not known until expanded, so it cannot be hidden upfront.
const UNFILED_PROJECT: Project = {
  id: UNFILED_ID,
  name: "Unfiled",
  sourceKind: "blank",
  rootPath: null,
  workspaceId: DEFAULT_WORKSPACE_ID,
  status: "active",
};

export function ProjectTree({
  projects,
  handlers,
}: {
  projects: Project[];
  handlers: ProjectTreeHandlers;
}) {
  const namedProjects = projects.filter((p) => p.id !== UNFILED_ID);
  const namedProjectIds = namedProjects.map((p) => p.id);

  const rowFor = (project: Project, projectIds: string[]) => (
    <ProjectRow
      key={project.id}
      project={project}
      isDesktop={handlers.isDesktop}
      detached={handlers.isDetached(project.id)}
      editingId={handlers.editingId}
      projectIds={projectIds}
      onStartEdit={() => handlers.onStartEdit(project.id)}
      onStartEditSession={handlers.onStartEditSession}
      onCancelEdit={handlers.onCancelEdit}
      onRename={(newName) => handlers.onRenameProject(project.id, newName)}
      onRenameSession={handlers.onRenameSession}
      onReorderSessions={handlers.onReorderSessions}
      onReorderProjects={handlers.onReorderProjects}
      onNewSession={() => handlers.onNewSession(project.id)}
      onArchiveSession={handlers.onArchiveSession}
      onFocusDetached={() => handlers.onFocusDetached(project.id)}
    />
  );

  return (
    <div className="flex flex-col gap-3 py-1">
      {namedProjects.map((proj) => rowFor(proj, namedProjectIds))}
      {rowFor(UNFILED_PROJECT, [])}
    </div>
  );
}
