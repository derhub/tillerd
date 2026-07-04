import type { CommandView } from "@tillerd/client-bindings";

import { Copy, Pencil, Pin, PinOff, Trash2 } from "lucide-react";

import { EntityContextMenu } from "~/components/shell/EntityContextMenu";
import { InlineRenameInput } from "~/components/sidebar/InlineRenameInput";
import { Badge } from "~/components/ui/badge";
import { Tooltip, TooltipContent, TooltipTrigger } from "~/components/ui/tooltip";
import { cn } from "~/lib/utils";

const ROW_ACTION_CLASS =
  "opacity-0 group-hover:opacity-100 flex items-center justify-center w-6 h-6 rounded-sm transition-all duration-[var(--motion-fast)] ease-standard focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring";

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
          strokeWidth={2}
          aria-hidden
          data-testid="command-pinned-indicator"
          className="shrink-0 size-[var(--icon-sm)] text-muted-foreground/40"
        />
      )}

      <Badge variant="outline" className="shrink-0" data-testid="command-origin-badge">
        {command.origin}
      </Badge>

      {isDesktop && (
        <div className="flex items-center gap-0.5 shrink-0">
          {canEdit && (
            <Tooltip>
              <TooltipTrigger
                aria-label={`Edit ${command.name}`}
                onClick={(e) => {
                  e.stopPropagation();
                  onEdit(command.id);
                }}
                className={cn(
                  ROW_ACTION_CLASS,
                  "text-muted-foreground/50 hover:text-foreground hover:bg-muted",
                )}
              >
                <Pencil className="size-[var(--icon-sm)]" strokeWidth={2} />
              </TooltipTrigger>
              <TooltipContent>Edit</TooltipContent>
            </Tooltip>
          )}
          <Tooltip>
            <TooltipTrigger
              aria-label={`Duplicate ${command.name}`}
              onClick={(e) => {
                e.stopPropagation();
                onDuplicate(command.id);
              }}
              className={cn(
                ROW_ACTION_CLASS,
                "text-muted-foreground/50 hover:text-foreground hover:bg-muted",
              )}
            >
              <Copy className="size-[var(--icon-sm)]" strokeWidth={2} />
            </TooltipTrigger>
            <TooltipContent>Duplicate</TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger
              aria-label={command.pinned ? `Unpin ${command.name}` : `Pin ${command.name}`}
              onClick={(e) => {
                e.stopPropagation();
                if (command.pinned) onUnpin(command.id);
                else onPin(command.id);
              }}
              className={cn(
                ROW_ACTION_CLASS,
                "text-muted-foreground/50 hover:text-foreground hover:bg-muted",
              )}
            >
              {command.pinned ? (
                <PinOff className="size-[var(--icon-sm)]" strokeWidth={2} />
              ) : (
                <Pin className="size-[var(--icon-sm)]" strokeWidth={2} />
              )}
            </TooltipTrigger>
            <TooltipContent>{command.pinned ? "Unpin" : "Pin"}</TooltipContent>
          </Tooltip>
          {canEdit && (
            <Tooltip>
              <TooltipTrigger
                aria-label={`Delete ${command.name}`}
                onClick={(e) => {
                  e.stopPropagation();
                  onRequestDelete(command.id, command.name);
                }}
                className={cn(
                  ROW_ACTION_CLASS,
                  "text-muted-foreground/50 hover:text-destructive hover:bg-destructive/10",
                )}
              >
                <Trash2 className="size-[var(--icon-sm)]" strokeWidth={2} />
              </TooltipTrigger>
              <TooltipContent>Delete</TooltipContent>
            </Tooltip>
          )}
        </div>
      )}
    </EntityContextMenu>
  );
}
