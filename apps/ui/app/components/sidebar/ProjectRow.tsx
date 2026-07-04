import type { Project } from "@tillerd/client-bindings";

import { ArrowUpRight, ChevronDown, ChevronRight, Pin, Plus } from "lucide-react";
import React from "react";

import { EntityContextMenu } from "~/components/shell/EntityContextMenu";
import type { DeleteTarget } from "~/components/sidebar/DeleteDialog";
import { InlineRenameInput } from "~/components/sidebar/InlineRenameInput";
import { ProjectSessions } from "~/components/sidebar/ProjectSessions";
import { useTreeNav } from "~/components/sidebar/ProjectTree";
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
  onRestoreSession,
  onRequestDelete,
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
  onArchiveSession: (id: string) => void;
  onRestoreSession: (id: string) => void;
  onRequestDelete: (target: DeleteTarget) => void;
  onFocusDetached: () => void;
}) {
  const [dragOver, setDragOver] = React.useState(false);
  const [expanded, setExpanded] = useProjectExpanded(project.id);
  const { activeId, setActiveId } = useTreeNav();

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
        args={{ label: project.name, workspaceId: project.workspaceId }}
        role="treeitem"
        aria-level={1}
        aria-expanded={expanded}
        aria-label={project.name}
        data-tree-id={project.id}
        data-level="1"
        data-expanded={expanded}
        tabIndex={activeId === project.id ? 0 : -1}
        onFocus={() => setActiveId(project.id)}
        guards={{
          "menu.canRename": !isUnfiled,
          "menu.canDuplicate": !isUnfiled,
          "menu.canPin": !isUnfiled,
          "menu.canMove": can("project", "move", project),
          "menu.canArchive": can("project", "archive", project),
          "menu.canDelete": can("project", "discard", project),
          "menu.pinned": project.pinned,
        }}
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
          tabIndex={-1}
          onClick={() => setExpanded(!expanded)}
          aria-expanded={expanded}
          aria-label={expanded ? `Collapse ${project.name}` : `Expand ${project.name}`}
          data-testid="project-expand"
          data-project-id={project.id}
          className="flex items-center p-0.5 rounded-sm text-muted-foreground/50 hover:text-foreground hover:bg-muted"
        >
          {expanded ? (
            <ChevronDown strokeWidth={2} className="size-[var(--icon-sm)]" />
          ) : (
            <ChevronRight strokeWidth={2} className="size-[var(--icon-sm)]" />
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
        {project.pinned && (
          <Pin
            strokeWidth={2}
            aria-hidden
            data-testid="project-pinned-indicator"
            className="shrink-0 text-muted-foreground/40 size-[var(--icon-sm)]"
          />
        )}
        {detached && (
          <button
            type="button"
            tabIndex={-1}
            onClick={onFocusDetached}
            aria-label={`Re-attach ${project.name}`}
            title={`${project.name} is in another window — click to re-attach`}
            data-testid="project-detached-indicator"
            className={cn(
              "flex items-center p-0.5 rounded-sm transition-colors duration-[var(--motion-fast)] ease-standard",
              "text-amber-500/80 hover:text-amber-400 hover:bg-muted",
            )}
          >
            <ArrowUpRight strokeWidth={2} className="size-[var(--icon-sm)]" />
          </button>
        )}
        {isDesktop && (
          <button
            type="button"
            tabIndex={-1}
            onClick={onNewSession}
            className={cn(
              "flex items-center p-0.5 rounded-sm transition-colors duration-[var(--motion-fast)] ease-standard",
              "text-muted-foreground/50 hover:text-foreground hover:bg-muted",
            )}
            title={`New session in ${project.name}`}
          >
            <Plus strokeWidth={2} className="size-[var(--icon-sm)]" />
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
          onRestoreSession={onRestoreSession}
          onRequestDelete={onRequestDelete}
        />
      )}
    </div>
  );
}
