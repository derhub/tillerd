import { Pencil, Trash2 } from "lucide-react";

import { EntityContextMenu } from "~/components/shell/EntityContextMenu";
import { cn } from "~/lib/utils";

// One project launch-template row. Unlike a library template this carries no
// name (spec: label derived from the spec, e.g. the first item's command) and
// no pin/export -- only edit (opens the visual spec editor) and discard.
export function LaunchTemplateRow({
  id,
  label,
  isDesktop,
  onEdit,
  onRequestDiscard,
}: {
  id: string;
  label: string;
  isDesktop: boolean;
  onEdit: (id: string) => void;
  onRequestDiscard: (id: string, label: string) => void;
}) {
  return (
    <EntityContextMenu
      entityId={id}
      entityKind="launchTemplate"
      args={{ label }}
      disabled={!isDesktop}
      className="group flex items-center gap-2 h-8 px-3 rounded-sm"
      data-testid="launch-template-row"
      data-launch-template-id={id}
    >
      <span
        className="flex-1 min-w-0 truncate text-[0.833rem] text-foreground"
        data-testid="launch-template-label"
      >
        {label}
      </span>

      {isDesktop && (
        <div className="flex items-center gap-0.5 shrink-0">
          <button
            type="button"
            aria-label={`Edit ${label}`}
            title="Edit"
            onClick={(e) => {
              e.stopPropagation();
              onEdit(id);
            }}
            className={cn(
              "opacity-0 group-hover:opacity-100 flex items-center justify-center w-6 h-6 rounded-sm transition-all duration-[var(--motion-fast)] ease-standard",
              "text-muted-foreground/50 hover:text-foreground hover:bg-muted",
            )}
          >
            <Pencil size={11} strokeWidth={2} />
          </button>
          <button
            type="button"
            aria-label={`Discard ${label}`}
            title="Discard"
            onClick={(e) => {
              e.stopPropagation();
              onRequestDiscard(id, label);
            }}
            className={cn(
              "opacity-0 group-hover:opacity-100 flex items-center justify-center w-6 h-6 rounded-sm transition-all duration-[var(--motion-fast)] ease-standard",
              "text-muted-foreground/50 hover:text-destructive hover:bg-destructive/10",
            )}
          >
            <Trash2 size={11} strokeWidth={2} />
          </button>
        </div>
      )}
    </EntityContextMenu>
  );
}
