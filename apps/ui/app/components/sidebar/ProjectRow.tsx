import type { Project } from "@tillerd/client-bindings";

import { ArrowUpRight, ChevronDown, ChevronRight, Plus } from "lucide-react";
import React from "react";

import { EntityContextMenu } from "~/components/shell/EntityContextMenu";
import { InlineRenameInput } from "~/components/sidebar/InlineRenameInput";
import { ProjectSessions } from "~/components/sidebar/ProjectSessions";
import { DRAG_PROJECT, UNFILED_ID } from "~/components/sidebar/sidebar-data";
import { reorderByDrop } from "~/lib/reorder";
import { can } from "~/lib/stateModel";
import { useProjectExpanded } from "~/lib/store";
import { cn } from "~/lib/utils";

// ProjectSessions mounts ONLY while expanded -- a collapsed project fetches nothing.
export function ProjectRow({
  project,
  isDesktop,
  detached,
  editingId,
  onStartEdit,
  onStartEditSession,
  onCancelEdit,
  onRename,
  onRenameSession,
  onReorderSessions,
  onReorderProjects,
  projectIds,
  onNewSession,
  onArchiveSession,
  onFocusDetached,
}: {
  project: Project;
  isDesktop: boolean;
  detached: boolean;
  editingId: string | null;
  onStartEdit: () => void;
  onStartEditSession: (sessionId: string) => void;
  onCancelEdit: () => void;
  onRename: (newName: string) => void;
  onRenameSession: (sessionId: string, newName: string) => void;
  onReorderSessions: (orderedIds: string[]) => void;
  onReorderProjects: (orderedIds: string[]) => void;
  projectIds: string[];
  onNewSession: () => void;
  onArchiveSession: (id: string, currentPath: string) => void;
  onFocusDetached: () => void;
}) {
  const [dragOver, setDragOver] = React.useState(false);
  const [expanded, setExpanded] = useProjectExpanded(project.id);

  const isUnfiled = project.id === UNFILED_ID;
  const isEditing = editingId === project.id;
  // Unfiled is pinned last and cannot be dragged or dropped onto.
  const draggable = isDesktop && !isUnfiled;

  const handleProjectDrop = (e: React.DragEvent) => {
    setDragOver(false);
    const sourceId = e.dataTransfer.getData(DRAG_PROJECT);
    if (!sourceId || isUnfiled) return;
    const next = reorderByDrop(projectIds, sourceId, project.id);
    if (next !== projectIds) onReorderProjects(next);
  };

  return (
    <div>
      <EntityContextMenu
        entityId={project.id}
        entityKind="project"
        args={{ label: project.name }}
        guards={{ "menu.canRename": !isUnfiled, "menu.canDelete": can("project", "discard", project) }}
        disabled={!isDesktop}
        draggable={draggable}
        onDragStart={
          draggable
            ? (e) => {
                e.dataTransfer.setData(DRAG_PROJECT, project.id);
                e.dataTransfer.effectAllowed = "move";
              }
            : undefined
        }
        onDragOver={
          draggable
            ? (e) => {
                if (e.dataTransfer.types.includes(DRAG_PROJECT)) {
                  e.preventDefault();
                  setDragOver(true);
                }
              }
            : undefined
        }
        onDragLeave={() => setDragOver(false)}
        onDrop={draggable ? handleProjectDrop : undefined}
        className={cn(
          "flex items-center gap-1 px-3 mb-0.5",
          dragOver && "ring-1 ring-ring rounded-sm",
        )}
      >
        <button
          type="button"
          onClick={() => setExpanded(!expanded)}
          aria-expanded={expanded}
          aria-label={expanded ? `Collapse ${project.name}` : `Expand ${project.name}`}
          data-testid="project-expand"
          data-project-id={project.id}
          className="flex items-center p-0.5 rounded-sm text-muted-foreground/50 hover:text-foreground hover:bg-muted"
        >
          {expanded ? (
            <ChevronDown size={10} strokeWidth={2} />
          ) : (
            <ChevronRight size={10} strokeWidth={2} />
          )}
        </button>
        {isEditing ? (
          <InlineRenameInput
            initialValue={project.name}
            onConfirm={onRename}
            onCancel={onCancelEdit}
            isProject={true}
          />
        ) : (
          <span
            onDoubleClick={isUnfiled ? undefined : onStartEdit}
            data-testid="project-name"
            data-project-id={project.id}
            className="text-[0.75rem] font-medium text-muted-foreground/70 uppercase tracking-wider truncate flex-1 cursor-text"
          >
            {project.name}
          </span>
        )}
        {detached && (
          <button
            type="button"
            onClick={onFocusDetached}
            aria-label={`Re-attach ${project.name}`}
            title={`${project.name} is in another window — click to re-attach`}
            data-testid="project-detached-indicator"
            className={cn(
              "flex items-center p-0.5 rounded-sm transition-colors duration-[var(--motion-fast)] ease-standard",
              "text-amber-500/80 hover:text-amber-400 hover:bg-muted",
            )}
          >
            <ArrowUpRight size={10} strokeWidth={2} />
          </button>
        )}
        {isDesktop && (
          <button
            type="button"
            onClick={onNewSession}
            className={cn(
              "flex items-center p-0.5 rounded-sm transition-colors duration-[var(--motion-fast)] ease-standard",
              "text-muted-foreground/50 hover:text-foreground hover:bg-muted",
            )}
            title={`New session in ${project.name}`}
          >
            <Plus size={10} strokeWidth={2} />
          </button>
        )}
      </EntityContextMenu>

      {expanded && (
        <ProjectSessions
          projectId={project.id}
          isDesktop={isDesktop}
          editingId={editingId}
          onStartEditSession={onStartEditSession}
          onCancelEdit={onCancelEdit}
          onRenameSession={onRenameSession}
          onReorderSessions={onReorderSessions}
          onArchiveSession={onArchiveSession}
        />
      )}
    </div>
  );
}
