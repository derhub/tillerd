import type { CommandView } from "@tillerd/client-bindings";

import { Copy, Pencil, Pin, PinOff, Trash2 } from "lucide-react";

import { EntityContextMenu } from "~/components/shell/EntityContextMenu";
import { InlineRenameInput } from "~/components/sidebar/InlineRenameInput";
import { Badge } from "~/components/ui/badge";
import { cn } from "~/lib/utils";

// One command-library row. Prebuilt rows never offer Edit/Rename/Delete (`canEdit`
// gates the def's `when` and this row's own hover buttons identically -- the
// context menu and the hover actions must agree on what a row can do).
export function CommandRow({
  command,
  isDesktop,
  isRenaming,
  onStartRename,
  onConfirmRename,
  onCancelRename,
  onEdit,
  onDuplicate,
  onPin,
  onUnpin,
  onRequestDelete,
}: {
  command: CommandView;
  isDesktop: boolean;
  isRenaming: boolean;
  onStartRename: (id: string) => void;
  onConfirmRename: (name: string) => void;
  onCancelRename: () => void;
  onEdit: (id: string) => void;
  onDuplicate: (id: string) => void;
  onPin: (id: string) => void;
  onUnpin: (id: string) => void;
  onRequestDelete: (id: string, name: string) => void;
}) {
  const canEdit = command.origin === "custom";

  return (
    <EntityContextMenu
      entityId={command.id}
      entityKind="command"
      args={{ label: command.name }}
      guards={{ "menu.canEdit": canEdit, "menu.pinned": command.pinned }}
      disabled={!isDesktop}
      className="group flex items-center gap-2 h-8 px-3 rounded-sm"
      data-testid="command-row"
      data-command-id={command.id}
      data-command-origin={command.origin}
    >
      <div className="flex-1 min-w-0 flex items-center gap-2">
        {isRenaming ? (
          <InlineRenameInput
            initialValue={command.name}
            onConfirm={onConfirmRename}
            onCancel={onCancelRename}
            isProject
          />
        ) : (
          <span
            data-testid="command-name"
            onDoubleClick={isDesktop && canEdit ? () => onStartRename(command.id) : undefined}
            className={cn("truncate text-[0.833rem] text-foreground", canEdit && "cursor-text")}
          >
            {command.name}
          </span>
        )}
        <span className="truncate text-[0.75rem] text-muted-foreground/60">{command.cli}</span>
      </div>

      {command.pinned && (
        <Pin
          size={9}
          strokeWidth={2}
          aria-hidden
          data-testid="command-pinned-indicator"
          className="shrink-0 text-muted-foreground/40"
        />
      )}

      <Badge variant="outline" className="shrink-0 text-[0.7rem]" data-testid="command-origin-badge">
        {command.origin}
      </Badge>

      {isDesktop && (
        <div className="flex items-center gap-0.5 shrink-0">
          {canEdit && (
            <button
              type="button"
              aria-label={`Edit ${command.name}`}
              title="Edit"
              onClick={(e) => {
                e.stopPropagation();
                onEdit(command.id);
              }}
              className={cn(
                "opacity-0 group-hover:opacity-100 flex items-center justify-center w-6 h-6 rounded-sm transition-all duration-[var(--motion-fast)] ease-standard",
                "text-muted-foreground/50 hover:text-foreground hover:bg-muted",
              )}
            >
              <Pencil size={11} strokeWidth={2} />
            </button>
          )}
          <button
            type="button"
            aria-label={`Duplicate ${command.name}`}
            title="Duplicate"
            onClick={(e) => {
              e.stopPropagation();
              onDuplicate(command.id);
            }}
            className={cn(
              "opacity-0 group-hover:opacity-100 flex items-center justify-center w-6 h-6 rounded-sm transition-all duration-[var(--motion-fast)] ease-standard",
              "text-muted-foreground/50 hover:text-foreground hover:bg-muted",
            )}
          >
            <Copy size={11} strokeWidth={2} />
          </button>
          <button
            type="button"
            aria-label={command.pinned ? `Unpin ${command.name}` : `Pin ${command.name}`}
            title={command.pinned ? "Unpin" : "Pin"}
            onClick={(e) => {
              e.stopPropagation();
              if (command.pinned) onUnpin(command.id);
              else onPin(command.id);
            }}
            className={cn(
              "opacity-0 group-hover:opacity-100 flex items-center justify-center w-6 h-6 rounded-sm transition-all duration-[var(--motion-fast)] ease-standard",
              "text-muted-foreground/50 hover:text-foreground hover:bg-muted",
            )}
          >
            {command.pinned ? <PinOff size={11} strokeWidth={2} /> : <Pin size={11} strokeWidth={2} />}
          </button>
          {canEdit && (
            <button
              type="button"
              aria-label={`Delete ${command.name}`}
              title="Delete"
              onClick={(e) => {
                e.stopPropagation();
                onRequestDelete(command.id, command.name);
              }}
              className={cn(
                "opacity-0 group-hover:opacity-100 flex items-center justify-center w-6 h-6 rounded-sm transition-all duration-[var(--motion-fast)] ease-standard",
                "text-muted-foreground/50 hover:text-destructive hover:bg-destructive/10",
              )}
            >
              <Trash2 size={11} strokeWidth={2} />
            </button>
          )}
        </div>
      )}
    </EntityContextMenu>
  );
}
