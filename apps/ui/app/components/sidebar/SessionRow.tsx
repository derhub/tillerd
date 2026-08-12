import type { Session } from "@tillerd/client-bindings";

import { Link, useRouterState } from "@tanstack/react-router";
import { Archive, Pin } from "lucide-react";
import React from "react";

import { EntityContextMenu } from "~/components/shell/EntityContextMenu";
import { InlineRenameInput } from "~/components/sidebar/InlineRenameInput";
import { useTreeNav } from "~/components/sidebar/ProjectTree";
import { DRAG_SESSION } from "~/components/sidebar/sidebar-data";
import { Tooltip, TooltipContent, TooltipTrigger } from "~/components/ui/tooltip";
import { sessionDisplayName } from "~/lib/panelTitle";
import { useSessionBadge, type SessionBadge } from "~/lib/sessionStatus";
import { cn } from "~/lib/utils";

// Surface-runtime badge colors (fed by the surface-status push channel). Idle is
// muted; running/starting/failed carry semantic hues.
const BADGE_CLASS: Record<SessionBadge, string> = {
  running: "bg-emerald-500/80",
  starting: "bg-amber-500/80",
  failed: "bg-red-500/80",
  idle: "bg-muted-foreground/30",
};

function ActiveSessionLink({
  sessionId,
  isActive,
  onDoubleClick,
  children,
}: {
  sessionId: string;
  isActive: boolean;
  onDoubleClick?: () => void;
  children: React.ReactNode;
}) {
  return (
    <Link
      to={`/session/${sessionId}` as never}
      onDoubleClick={onDoubleClick}
      // The treeitem row owns focus (roving tabindex); the link is activated via
      // its Enter handler, so it stays out of the tab order.
      tabIndex={-1}
      className={cn(
        "flex items-center gap-2 flex-1 h-7 text-[0.917rem] rounded-sm transition-colors duration-[var(--motion-fast)] ease-standard min-w-0",
        isActive
          ? "bg-muted text-foreground"
          : "text-muted-foreground hover:text-foreground hover:bg-muted/50",
      )}
    >
      {children}
    </Link>
  );
}

export function SessionRow({
  session,
  projectId,
  isDesktop,
  isEditing,
  onStartEdit,
  onCancelEdit,
  onRename,
  onArchive,
  onSessionDrop,
}: {
  session: Session;
  projectId: string;
  isDesktop: boolean;
  isEditing: boolean;
  onStartEdit: () => void;
  onCancelEdit: () => void;
  onRename: (newName: string) => void;
  onArchive: () => void;
  onSessionDrop: (sourceId: string, targetId: string) => void;
}) {
  const label = sessionDisplayName(session.title, session.id);
  const [dragOver, setDragOver] = React.useState(false);
  const badge = useSessionBadge(session.id);
  const { activeId, setActiveId } = useTreeNav();
  const routerState = useRouterState();
  const isActive = routerState.location.pathname === `/session/${session.id}`;

  if (isEditing) {
    return (
      <div className="flex items-center gap-1 px-3">
        <InlineRenameInput
          initialValue={session.title}
          onConfirm={onRename}
          onCancel={onCancelEdit}
        />
      </div>
    );
  }

  return (
    <EntityContextMenu
      entityId={session.id}
      entityKind="session"
      args={{ label, projectId }}
      role="treeitem"
      aria-level={2}
      aria-selected={isActive}
      aria-label={label}
      data-tree-id={session.id}
      data-level="2"
      data-parent-id={projectId}
      tabIndex={activeId === session.id ? 0 : -1}
      onFocus={() => setActiveId(session.id)}
      guards={{ "menu.pinned": session.pinned }}
      disabled={!isDesktop}
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
      className={cn(
        "group flex items-center gap-1 px-3 rounded-sm",
        dragOver && "ring-1 ring-ring",
      )}
    >
      <ActiveSessionLink sessionId={session.id} isActive={isActive} onDoubleClick={onStartEdit}>
        <span
          data-testid="session-status"
          data-status={badge}
          className={cn("w-1.5 h-1.5 rounded-full shrink-0", BADGE_CLASS[badge])}
        />
        <span className="truncate text-[0.833rem]">{label}</span>
      </ActiveSessionLink>

      {session.pinned && (
        <Pin
          strokeWidth={2}
          aria-hidden
          data-testid="session-pinned-indicator"
          className="shrink-0 text-muted-foreground/40 size-[var(--icon-sm)]"
        />
      )}
      {isDesktop && (
        <Tooltip>
          <TooltipTrigger
            type="button"
            tabIndex={0}
            onClick={(e) => {
              e.stopPropagation();
              onArchive();
            }}
            aria-label={`Archive ${label}`}
            className={cn(
              "opacity-0 group-hover:opacity-100 focus-visible:opacity-100 flex items-center p-0.5 rounded-sm transition-all duration-[var(--motion-fast)] ease-standard",
              "text-muted-foreground/50 hover:text-foreground hover:bg-muted focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
            )}
          >
            <Archive strokeWidth={2} className="size-[var(--icon-sm)]" />
          </TooltipTrigger>
          <TooltipContent>Archive {label}</TooltipContent>
        </Tooltip>
      )}
    </EntityContextMenu>
  );
}
