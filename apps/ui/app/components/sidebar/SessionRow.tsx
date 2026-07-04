import type { Session } from "@tillerd/client-bindings";

import { Link, useRouterState } from "@tanstack/react-router";
import { Archive } from "lucide-react";
import React from "react";

import { EntityContextMenu } from "~/components/shell/EntityContextMenu";
import { InlineRenameInput } from "~/components/sidebar/InlineRenameInput";
import { DRAG_SESSION } from "~/components/sidebar/sidebar-data";
import { cn } from "~/lib/utils";

function ActiveSessionLink({
  sessionId,
  onDoubleClick,
  children,
}: {
  sessionId: string;
  onDoubleClick?: () => void;
  children: React.ReactNode;
}) {
  const state = useRouterState();
  const isActive = state.location.pathname === `/session/${sessionId}`;
  return (
    <Link
      to={`/session/${sessionId}` as never}
      onDoubleClick={onDoubleClick}
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
  isDesktop,
  isEditing,
  onStartEdit,
  onCancelEdit,
  onRename,
  onArchive,
  onSessionDrop,
}: {
  session: Session;
  isDesktop: boolean;
  isEditing: boolean;
  onStartEdit: () => void;
  onCancelEdit: () => void;
  onRename: (newName: string) => void;
  onArchive: () => void;
  onSessionDrop: (sourceId: string, targetId: string) => void;
}) {
  const label = session.title || session.id.slice(0, 8);
  const [dragOver, setDragOver] = React.useState(false);

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
      args={{ label }}
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
      <ActiveSessionLink sessionId={session.id} onDoubleClick={onStartEdit}>
        <span className="w-1.5 h-1.5 rounded-full shrink-0 bg-emerald-500/80" />
        <span className="truncate text-[0.833rem]">{label}</span>
      </ActiveSessionLink>

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
    </EntityContextMenu>
  );
}
