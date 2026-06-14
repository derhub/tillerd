import { useState, useEffect, useCallback, useRef } from "react";
import { NavLink, useNavigate } from "react-router";
import {
  Plus,
  FolderPlus,
  Archive,
  ArrowUpRight,
  ExternalLink,
  Trash2,
  Pencil,
} from "lucide-react";
import { ScrollArea } from "~/components/ui/scroll-area";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogTitle,
} from "~/components/ui/alert-dialog";
import { cn } from "~/lib/utils";
import { useDesktopHost } from "~/lib/useDesktopHost";
import {
  focusSelf,
  focusWindow,
  onReattachProject,
  openWindow,
  projectLabel,
  projectQuery,
} from "~/lib/windows";
import { InlineRenameInput } from "~/components/InlineRenameInput";
import { reorderByDrop } from "~/lib/reorder";

import type { Project, Session } from "@tillerd/sdk/orchestrator";

const UNFILED_ID = "00000000-0000-0000-0000-000000000000";

/** Fetch projects and sessions from the orchestrator transport (desktop) or HTTP API (web). */
function useSidebarData() {
  const host = useDesktopHost();
  const [projects, setProjects] = useState<Project[]>([]);
  const [sessions, setSessions] = useState<Session[]>([]);

  const refresh = useCallback(async () => {
    if (host.status === "ready") {
      try {
        const client = host.orchestratorClient;
        const [ps, ss] = await Promise.all([client.listProjects(), client.listSessions()]);
        setProjects(ps);
        setSessions(ss);
      } catch {
        // non-fatal; keep stale data
      }
    }
    // web path: data comes via loader revalidation; sidebar stays read-only here
  }, [host]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return { projects, sessions, refresh };
}

export function SessionSidebar() {
  const host = useDesktopHost();
  const navigate = useNavigate();
  const { projects, sessions, refresh } = useSidebarData();
  const [detachedProjects, setDetachedProjects] = useState<Set<string>>(() => new Set());
  const [editingId, setEditingId] = useState<string | null>(null);
  const [deleteConfirm, setDeleteConfirm] = useState<{
    id: string;
    name: string;
    kind: "project" | "session";
  } | null>(null);

  const handleOpenInNewWindow = useCallback((projectId: string, firstSessionId: string | null) => {
    void openWindow(projectLabel(projectId), projectQuery(projectId, firstSessionId));
    setDetachedProjects((prev) => new Set(prev).add(projectId));
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void onReattachProject(({ projectId }) => {
      setDetachedProjects((prev) => {
        if (!prev.has(projectId)) return prev;
        const next = new Set(prev);
        next.delete(projectId);
        return next;
      });
      void focusSelf();
    }).then((u) => (unlisten = u));
    return () => unlisten?.();
  }, []);

  const handleNewProject = useCallback(async () => {
    if (host.status !== "ready") return;
    const name = window.prompt("Project name (leave blank for a blank project):") ?? "";
    const proj = await host.orchestratorClient.createProject({
      sourceKind: "blank",
      name: name.trim() || undefined,
    });
    await refresh();
    // Navigate to first session of new project (not yet created)
    void navigate(`/`);
    // Create a default session under the new project
    const sess = await host.orchestratorClient.createSession({
      projectId: proj.id,
      titleSource: "agent-title",
    });
    await refresh();
    void navigate(`/session/${sess.id}`);
  }, [host, navigate, refresh]);

  const handleNewSession = useCallback(
    async (projectId: string) => {
      if (host.status !== "ready") return;
      const sess = await host.orchestratorClient.createSession({
        projectId,
        titleSource: "agent-title",
      });
      await refresh();
      void navigate(`/session/${sess.id}`);
    },
    [host, navigate, refresh],
  );

  const handleArchiveSession = useCallback(
    async (sessId: string, currentPath: string) => {
      if (host.status !== "ready") return;
      await host.orchestratorClient.archiveSession({ id: sessId });
      await refresh();
      if (currentPath === `/session/${sessId}`) {
        void navigate("/");
      }
    },
    [host, navigate, refresh],
  );

  const handleRenameProject = useCallback(
    async (projectId: string, newName: string) => {
      if (host.status !== "ready") return;
      await host.orchestratorClient.renameProject({ id: projectId, name: newName });
      await refresh();
      setEditingId(null);
    },
    [host, refresh],
  );

  const handleRenameSession = useCallback(
    async (sessId: string, newName: string) => {
      if (host.status !== "ready") return;
      await host.orchestratorClient.renameSession({ id: sessId, title: newName });
      await refresh();
      setEditingId(null);
    },
    [host, refresh],
  );

  const handleConfirmDelete = useCallback(async () => {
    if (host.status !== "ready" || !deleteConfirm) return;
    const { id, kind } = deleteConfirm;
    if (kind === "project") {
      await host.orchestratorClient.deleteProject({ id });
    } else {
      await host.orchestratorClient.deleteSession({ id });
    }
    await refresh();
    setDeleteConfirm(null);
    if (kind === "session" && window.location.pathname === `/session/${id}`) {
      void navigate("/");
    } else if (kind === "project") {
      void navigate("/");
    }
  }, [host, deleteConfirm, navigate, refresh]);

  // Persist a full ordered list by renumbering every item 0..N-1 so the stored sort_order is a clean
  // total order (no interleave with the rowid fallback once all rows carry an explicit value).
  const handleReorderProjects = useCallback(
    async (orderedIds: string[]) => {
      if (host.status !== "ready") return;
      await Promise.all(
        orderedIds.map((id, i) => host.orchestratorClient.reorderProject({ id, sortOrder: i })),
      );
      await refresh();
    },
    [host, refresh],
  );

  const handleReorderSessions = useCallback(
    async (orderedIds: string[]) => {
      if (host.status !== "ready") return;
      await Promise.all(
        orderedIds.map((id, i) => host.orchestratorClient.reorderSession({ id, sortOrder: i })),
      );
      await refresh();
    },
    [host, refresh],
  );

  // Group sessions by projectId
  const sessionsByProject = new Map<string, Session[]>();
  for (const s of sessions) {
    const group = sessionsByProject.get(s.projectId) ?? [];
    group.push(s);
    sessionsByProject.set(s.projectId, group);
  }

  // Build ordered project list: named projects first, Unfiled last (hidden when empty)
  const namedProjects = projects.filter((p) => p.id !== UNFILED_ID);
  const unfiledSessions = sessionsByProject.get(UNFILED_ID) ?? [];

  const isDesktop = host.status === "ready";

  return (
    <div className="flex flex-col h-full">
      {/* Delete confirmation dialog */}
      {deleteConfirm && (
        <AlertDialog
          open={true}
          onOpenChange={(open) => {
            if (!open) setDeleteConfirm(null);
          }}
        >
          <AlertDialogContent>
            <AlertDialogTitle>Delete {deleteConfirm.name}?</AlertDialogTitle>
            <AlertDialogDescription>
              {deleteConfirm.kind === "project"
                ? "This will permanently delete the project and all its sessions."
                : "This will permanently delete the session and terminate its PTYs."}
            </AlertDialogDescription>
            <div className="flex gap-2 justify-end">
              <AlertDialogCancel onClick={() => setDeleteConfirm(null)}>Cancel</AlertDialogCancel>
              <AlertDialogAction
                onClick={() => void handleConfirmDelete()}
                className="bg-destructive hover:bg-destructive/90"
              >
                Delete
              </AlertDialogAction>
            </div>
          </AlertDialogContent>
        </AlertDialog>
      )}

      {/* Top controls */}
      <div className="px-3 pt-2 pb-1 shrink-0 flex items-center gap-1">
        {isDesktop && (
          <button
            type="button"
            onClick={() => void handleNewProject()}
            className={cn(
              "flex items-center gap-1.5 px-2 h-6 text-[0.917rem] rounded-sm transition-colors duration-[var(--motion-fast)] ease-standard",
              "text-muted-foreground hover:text-foreground hover:bg-muted",
            )}
            title="New project"
          >
            <FolderPlus size={11} strokeWidth={2} />
            <span>New project</span>
          </button>
        )}
      </div>

      <ScrollArea className="flex-1 min-h-0">
        {sessions.length === 0 ? (
          <p className="px-3 py-3 text-[0.917rem] text-muted-foreground/50 italic">
            No active sessions
          </p>
        ) : (
          <div className="flex flex-col gap-3 py-1">
            {namedProjects.map((proj) => {
              const projSessions = sessionsByProject.get(proj.id) ?? [];
              return (
                <ProjectGroup
                  key={proj.id}
                  project={proj}
                  sessions={projSessions}
                  isDesktop={isDesktop}
                  detached={detachedProjects.has(proj.id)}
                  isEditing={editingId === proj.id}
                  editingId={editingId}
                  onStartEdit={() => setEditingId(proj.id)}
                  onStartEditSession={(sid) => setEditingId(sid)}
                  onRename={(newName) => void handleRenameProject(proj.id, newName)}
                  onRenameSession={handleRenameSession}
                  onDelete={() =>
                    setDeleteConfirm({ id: proj.id, name: proj.name, kind: "project" })
                  }
                  onDeleteSession={(sid, name) =>
                    setDeleteConfirm({ id: sid, name, kind: "session" })
                  }
                  onReorderSessions={handleReorderSessions}
                  onReorderProjects={handleReorderProjects}
                  projectIds={namedProjects.map((p) => p.id)}
                  onNewSession={() => void handleNewSession(proj.id)}
                  onArchiveSession={handleArchiveSession}
                  onOpenInNewWindow={() =>
                    handleOpenInNewWindow(proj.id, projSessions[0]?.id ?? null)
                  }
                  onFocusDetached={() => void focusWindow(projectLabel(proj.id))}
                />
              );
            })}

            {/* Unfiled: shown last, hidden when empty */}
            {unfiledSessions.length > 0 && (
              <ProjectGroup
                key={UNFILED_ID}
                project={{ id: UNFILED_ID, name: "Unfiled", sourceKind: "blank", rootPath: null }}
                sessions={unfiledSessions}
                isDesktop={isDesktop}
                detached={detachedProjects.has(UNFILED_ID)}
                isEditing={editingId === UNFILED_ID}
                editingId={editingId}
                onStartEdit={() => setEditingId(UNFILED_ID)}
                onStartEditSession={(sid) => setEditingId(sid)}
                onRename={(newName) => void handleRenameProject(UNFILED_ID, newName)}
                onRenameSession={handleRenameSession}
                onDelete={() => {}}
                onDeleteSession={(sid, name) =>
                  setDeleteConfirm({ id: sid, name, kind: "session" })
                }
                onReorderSessions={handleReorderSessions}
                onReorderProjects={handleReorderProjects}
                projectIds={[]}
                onNewSession={() => void handleNewSession(UNFILED_ID)}
                onArchiveSession={handleArchiveSession}
                onOpenInNewWindow={() =>
                  handleOpenInNewWindow(UNFILED_ID, unfiledSessions[0]?.id ?? null)
                }
                onFocusDetached={() => void focusWindow(projectLabel(UNFILED_ID))}
              />
            )}
          </div>
        )}
      </ScrollArea>
    </div>
  );
}

// dataTransfer keys identifying a drag's payload kind and id.
const DRAG_PROJECT = "application/x-tillerd-project";
const DRAG_SESSION = "application/x-tillerd-session";

function ProjectGroup({
  project,
  sessions,
  isDesktop,
  detached,
  isEditing,
  editingId,
  onStartEdit,
  onStartEditSession,
  onRename,
  onRenameSession,
  onDelete,
  onDeleteSession,
  onReorderSessions,
  onReorderProjects,
  projectIds,
  onNewSession,
  onArchiveSession,
  onOpenInNewWindow,
  onFocusDetached,
}: {
  project: Project;
  sessions: Session[];
  isDesktop: boolean;
  detached: boolean;
  isEditing: boolean;
  editingId: string | null;
  onStartEdit: () => void;
  onStartEditSession: (sessionId: string) => void;
  onRename: (newName: string) => void;
  onRenameSession: (sessionId: string, newName: string) => void;
  onDelete: () => void;
  onDeleteSession: (sessionId: string, name: string) => void;
  onReorderSessions: (orderedIds: string[]) => void;
  onReorderProjects: (orderedIds: string[]) => void;
  projectIds: string[];
  onNewSession: () => void;
  onArchiveSession: (id: string, currentPath: string) => Promise<void>;
  onOpenInNewWindow: () => void;
  onFocusDetached: () => void;
}) {
  const [menuAt, setMenuAt] = useState<{ x: number; y: number } | null>(null);
  const [dragOver, setDragOver] = useState(false);
  const isUnfiled = project.id === UNFILED_ID;
  // Unfiled is never a reorder participant — it is pinned last and cannot be dragged or dropped onto.
  const draggable = isDesktop && !isUnfiled;

  // Drop another project's header onto this one to splice it into this slot.
  const handleProjectDrop = (e: React.DragEvent) => {
    setDragOver(false);
    const sourceId = e.dataTransfer.getData(DRAG_PROJECT);
    if (!sourceId || isUnfiled) return;
    const next = reorderByDrop(projectIds, sourceId, project.id);
    if (next !== projectIds) onReorderProjects(next);
  };

  // Drop a session onto another within the SAME project to reorder. A session from another project
  // is absent from this list, so `reorderByDrop` returns the input unchanged — cross-project rejected.
  const handleSessionDrop = (sourceId: string, targetId: string) => {
    const ids = sessions.map((s) => s.id);
    const next = reorderByDrop(ids, sourceId, targetId);
    if (next !== ids) onReorderSessions(next);
  };

  return (
    <div>
      {/* Project heading + add-session control */}
      <div
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
        onContextMenu={
          isDesktop
            ? (e) => {
                e.preventDefault();
                setMenuAt({ x: e.clientX, y: e.clientY });
              }
            : undefined
        }
      >
        {isEditing ? (
          <InlineRenameInput
            initialValue={project.name}
            onConfirm={onRename}
            onCancel={() => {}}
            isProject={true}
          />
        ) : (
          <span
            onDoubleClick={isUnfiled ? undefined : onStartEdit}
            data-testid="project-name"
            className="text-[0.75rem] font-medium text-muted-foreground/70 uppercase tracking-wider truncate flex-1 cursor-text"
          >
            {project.name}
          </span>
        )}
        {detached && (
          <button
            type="button"
            onClick={onFocusDetached}
            aria-label={`Focus ${project.name} window`}
            title={`${project.name} is open in another window`}
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
      </div>

      {menuAt && (
        <ProjectContextMenu
          at={menuAt}
          allowMutations={!isUnfiled}
          onClose={() => setMenuAt(null)}
          onRename={() => {
            onStartEdit();
            setMenuAt(null);
          }}
          onOpenInNewWindow={() => {
            onOpenInNewWindow();
            setMenuAt(null);
          }}
          onDelete={() => {
            onDelete();
            setMenuAt(null);
          }}
        />
      )}

      {/* Session rows */}
      <div className="flex flex-col gap-px">
        {sessions.map((s) => (
          <SessionRow
            key={s.id}
            session={s}
            isDesktop={isDesktop}
            isEditing={editingId === s.id}
            onStartEdit={() => onStartEditSession(s.id)}
            onRename={(newName) => onRenameSession(s.id, newName)}
            onArchive={() => void onArchiveSession(s.id, window.location.pathname)}
            onDelete={() => onDeleteSession(s.id, s.title || s.id.slice(0, 8))}
            onSessionDrop={handleSessionDrop}
          />
        ))}
      </div>
    </div>
  );
}

// Generic context-menu shell: closes on outside click or Escape, focuses the first item on open and
// supports Tab/Shift+Tab + arrow navigation between items.
function ContextMenuShell({
  at,
  onClose,
  children,
}: {
  at: { x: number; y: number };
  onClose: () => void;
  children: React.ReactNode;
}) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const items = () =>
      Array.from(ref.current?.querySelectorAll<HTMLButtonElement>("[role=menuitem]") ?? []);
    items()[0]?.focus();
    const onDown = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
        return;
      }
      if (e.key === "ArrowDown" || e.key === "ArrowUp") {
        e.preventDefault();
        const list = items();
        const i = list.indexOf(document.activeElement as HTMLButtonElement);
        const next =
          e.key === "ArrowDown"
            ? list[(i + 1) % list.length]
            : list[(i - 1 + list.length) % list.length];
        next?.focus();
      }
    };
    window.addEventListener("mousedown", onDown);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousedown", onDown);
      window.removeEventListener("keydown", onKey);
    };
  }, [onClose]);

  return (
    <div
      ref={ref}
      role="menu"
      style={{ position: "fixed", top: at.y, left: at.x, zIndex: 50 }}
      className="min-w-44 rounded-md border border-border/60 bg-popover p-1 shadow-md"
    >
      {children}
    </div>
  );
}

function MenuItem({ onClick, children }: { onClick: () => void; children: React.ReactNode }) {
  return (
    <button
      type="button"
      role="menuitem"
      onClick={onClick}
      className="flex w-full items-center gap-2 rounded-sm px-2 h-7 text-left text-[0.833rem] text-foreground hover:bg-muted focus:bg-muted focus:outline-none transition-colors duration-[var(--motion-fast)] ease-standard"
    >
      {children}
    </button>
  );
}

// Right-click menu with the full project action list. The Unfiled project hides mutating actions.
function ProjectContextMenu({
  at,
  allowMutations,
  onClose,
  onRename,
  onOpenInNewWindow,
  onDelete,
}: {
  at: { x: number; y: number };
  allowMutations: boolean;
  onClose: () => void;
  onRename: () => void;
  onOpenInNewWindow: () => void;
  onDelete: () => void;
}) {
  return (
    <ContextMenuShell at={at} onClose={onClose}>
      {allowMutations && (
        <MenuItem onClick={onRename}>
          <Pencil size={12} />
          <span>Rename</span>
        </MenuItem>
      )}
      <MenuItem onClick={onOpenInNewWindow}>
        <ExternalLink size={12} />
        <span>Open in new window</span>
      </MenuItem>
      {allowMutations && (
        <MenuItem onClick={onDelete}>
          <Trash2 size={12} />
          <span>Delete</span>
        </MenuItem>
      )}
    </ContextMenuShell>
  );
}

function SessionContextMenu({
  at,
  onClose,
  onRename,
  onArchive,
  onDelete,
}: {
  at: { x: number; y: number };
  onClose: () => void;
  onRename: () => void;
  onArchive: () => void;
  onDelete: () => void;
}) {
  return (
    <ContextMenuShell at={at} onClose={onClose}>
      <MenuItem onClick={onRename}>
        <Pencil size={12} />
        <span>Rename</span>
      </MenuItem>
      <MenuItem onClick={onArchive}>
        <Archive size={12} />
        <span>Archive</span>
      </MenuItem>
      <MenuItem onClick={onDelete}>
        <Trash2 size={12} />
        <span>Delete</span>
      </MenuItem>
    </ContextMenuShell>
  );
}

function SessionRow({
  session,
  isDesktop,
  isEditing,
  onStartEdit,
  onRename,
  onArchive,
  onDelete,
  onSessionDrop,
}: {
  session: Session;
  isDesktop: boolean;
  isEditing: boolean;
  onStartEdit: () => void;
  onRename: (newName: string) => void;
  onArchive: () => void;
  onDelete: () => void;
  onSessionDrop: (sourceId: string, targetId: string) => void;
}) {
  const label = session.title || session.id.slice(0, 8);
  const [menuAt, setMenuAt] = useState<{ x: number; y: number } | null>(null);
  const [dragOver, setDragOver] = useState(false);

  if (isEditing) {
    return (
      <div className="flex items-center gap-1 px-3">
        <InlineRenameInput initialValue={session.title} onConfirm={onRename} onCancel={() => {}} />
      </div>
    );
  }

  return (
    <div
      draggable={isDesktop}
      onDragStart={
        isDesktop
          ? (e) => {
              e.dataTransfer.setData(DRAG_SESSION, session.id);
              e.dataTransfer.effectAllowed = "move";
            }
          : undefined
      }
      onDragOver={
        isDesktop
          ? (e) => {
              if (e.dataTransfer.types.includes(DRAG_SESSION)) {
                e.preventDefault();
                setDragOver(true);
              }
            }
          : undefined
      }
      onDragLeave={() => setDragOver(false)}
      onDrop={
        isDesktop
          ? (e) => {
              setDragOver(false);
              const sourceId = e.dataTransfer.getData(DRAG_SESSION);
              if (sourceId) onSessionDrop(sourceId, session.id);
            }
          : undefined
      }
      onContextMenu={
        isDesktop
          ? (e) => {
              e.preventDefault();
              setMenuAt({ x: e.clientX, y: e.clientY });
            }
          : undefined
      }
      className={cn(
        "group flex items-center gap-1 px-3 rounded-sm",
        dragOver && "ring-1 ring-ring",
      )}
    >
      <NavLink
        to={`/session/${session.id}`}
        onDoubleClick={onStartEdit}
        className={({ isActive }) =>
          cn(
            "flex items-center gap-2 flex-1 h-7 text-[0.917rem] rounded-sm transition-colors duration-[var(--motion-fast)] ease-standard min-w-0",
            isActive
              ? "bg-muted text-foreground"
              : "text-muted-foreground hover:text-foreground hover:bg-muted/50",
          )
        }
      >
        <span className="w-1.5 h-1.5 rounded-full shrink-0 bg-emerald-500/80" />
        <span className="truncate text-[0.833rem]">{label}</span>
      </NavLink>

      {isDesktop && (
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            onArchive();
          }}
          className={cn(
            "opacity-0 group-hover:opacity-100 flex items-center p-0.5 rounded-sm transition-all duration-[var(--motion-fast)] ease-standard",
            "text-muted-foreground/50 hover:text-foreground hover:bg-muted",
          )}
          title="Archive session"
        >
          <Archive size={10} strokeWidth={2} />
        </button>
      )}

      {menuAt && (
        <SessionContextMenu
          at={menuAt}
          onClose={() => setMenuAt(null)}
          onRename={() => {
            onStartEdit();
            setMenuAt(null);
          }}
          onArchive={() => {
            onArchive();
            setMenuAt(null);
          }}
          onDelete={() => {
            onDelete();
            setMenuAt(null);
          }}
        />
      )}
    </div>
  );
}
