import type { Project } from "@tillerd/client-bindings";

import React from "react";

import { ArchivedRow, ArchivedSection } from "~/components/sidebar/ArchivedSection";
import type { DeleteTarget } from "~/components/sidebar/DeleteDialog";
import { ProjectRow } from "~/components/sidebar/ProjectRow";
import { DEFAULT_WORKSPACE_ID, UNFILED_ID } from "~/components/sidebar/sidebar-data";
import { setProjectExpanded } from "~/lib/store";

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
  onArchiveSession: (id: string) => void;
  onRestoreProject: (id: string) => void;
  onRestoreSession: (id: string) => void;
  onRequestDelete: (target: DeleteTarget) => void;
  onFocusDetached: (projectId: string) => void;
}

// Roving-tabindex owner for the sessions tree: exactly one visible row is
// tab-reachable, the rest are `tabIndex={-1}`; rows read `activeId` to decide.
interface TreeNav {
  activeId: string | null;
  setActiveId: (id: string) => void;
}
const TreeNavContext = React.createContext<TreeNav | null>(null);
export function useTreeNav(): TreeNav {
  const v = React.use(TreeNavContext);
  if (!v) throw new Error("useTreeNav must be used within a ProjectTree");
  return v;
}

// Unfiled always renders: emptiness is not known until expanded, so it cannot be hidden upfront.
const UNFILED_PROJECT: Project = {
  id: UNFILED_ID,
  name: "Unfiled",
  sourceKind: "blank",
  rootPath: null,
  workspaceId: DEFAULT_WORKSPACE_ID,
  status: "active",
  pinned: false,
};

export function ProjectTree({
  projects,
  handlers,
}: {
  projects: Project[];
  handlers: ProjectTreeHandlers;
}) {
  // The list read returns active + archived rows (status computed server-side);
  // the archived ones drop into their own collapsed section, out of the flow.
  const activeNamed = projects.filter((p) => p.id !== UNFILED_ID && p.status !== "archived");
  const archived = projects.filter((p) => p.status === "archived");
  const activeNamedIds = activeNamed.map((p) => p.id);

  const treeRef = React.useRef<HTMLDivElement>(null);
  // Seed the roving owner on the first project so Tab reaches the tree before any
  // arrow press; focus/keyboard moves then track the last-focused row.
  const [activeId, setActiveId] = React.useState<string | null>(
    () => activeNamed[0]?.id ?? UNFILED_ID,
  );
  const nav = React.useMemo<TreeNav>(() => ({ activeId, setActiveId }), [activeId]);

  // Self-heal a stale roving owner (its row unmounted, e.g. deleted elsewhere):
  // re-seed on the first row so the tree never falls out of the tab order.
  React.useEffect(() => {
    const el = treeRef.current;
    if (el && !el.querySelector('[role="treeitem"][tabindex="0"]')) {
      const first = el.querySelector<HTMLElement>("[role=\"treeitem\"]");
      if (first?.dataset.treeId) setActiveId(first.dataset.treeId);
    }
  });

  // Rows in visible (expanded) order: collapsed groups render nothing, so DOM
  // order already skips their hidden sessions.
  const visibleRows = (): HTMLElement[] =>
    treeRef.current ? [...treeRef.current.querySelectorAll<HTMLElement>('[role="treeitem"]')] : [];

  const focusRow = (el: HTMLElement | null | undefined) => {
    if (!el?.dataset.treeId) return;
    setActiveId(el.dataset.treeId);
    el.focus();
  };

  // One controller for the whole tree: every row's keydown bubbles here, so
  // expand/collapse (via the store) and open (via the row's own Link) are reused
  // rather than reimplemented per row.
  const onKeyDown = (e: React.KeyboardEvent) => {
    const target = e.target as HTMLElement;
    if (target.tagName === "INPUT" || target.tagName === "TEXTAREA") return; // inline rename owns its keys
    const current = target.closest<HTMLElement>('[role="treeitem"]');
    if (!current || !treeRef.current?.contains(current)) return;
    const id = current.dataset.treeId ?? "";
    const level = current.dataset.level;
    const expanded = current.dataset.expanded === "true";
    const rows = visibleRows();
    const idx = rows.indexOf(current);
    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        focusRow(rows[idx + 1]);
        break;
      case "ArrowUp":
        e.preventDefault();
        focusRow(rows[idx - 1]);
        break;
      case "ArrowRight":
        e.preventDefault();
        if (level === "1" && !expanded) setProjectExpanded(id, true);
        else focusRow(rows[idx + 1]);
        break;
      case "ArrowLeft":
        e.preventDefault();
        if (level === "1" && expanded) setProjectExpanded(id, false);
        else if (level === "2" && current.dataset.parentId)
          focusRow(treeRef.current.querySelector<HTMLElement>(
            `[role="treeitem"][data-tree-id="${current.dataset.parentId}"]`,
          ));
        break;
      case "Enter":
        e.preventDefault();
        if (level === "2") current.querySelector<HTMLElement>("a[href]")?.click();
        else if (level === "1") setProjectExpanded(id, !expanded);
        break;
    }
  };

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
      onRestoreSession={handlers.onRestoreSession}
      onRequestDelete={handlers.onRequestDelete}
      onFocusDetached={() => handlers.onFocusDetached(project.id)}
    />
  );

  return (
    <TreeNavContext.Provider value={nav}>
      <div className="flex flex-col gap-3 py-1">
        <div
          ref={treeRef}
          role="tree"
          aria-label="Sessions"
          onKeyDown={onKeyDown}
          className="flex flex-col gap-3"
        >
          {activeNamed.map((proj) => rowFor(proj, activeNamedIds))}
          {rowFor(UNFILED_PROJECT, [])}
        </div>
        {/* Archived projects sit outside the tree: they are a separate disclosure, not treeitems. */}
        <ArchivedSection count={archived.length}>
          {archived.map((p) => (
            <ArchivedRow
              key={p.id}
              name={p.name}
              onRestore={() => handlers.onRestoreProject(p.id)}
              onDelete={() => handlers.onRequestDelete({ id: p.id, name: p.name, kind: "project" })}
            />
          ))}
        </ArchivedSection>
      </div>
    </TreeNavContext.Provider>
  );
}
