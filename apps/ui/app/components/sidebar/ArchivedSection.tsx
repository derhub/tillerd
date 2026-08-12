import { ArchiveRestore, ChevronDown, ChevronRight, Trash2 } from "lucide-react";
import React from "react";

import { Tooltip, TooltipContent, TooltipTrigger } from "~/components/ui/tooltip";
import { cn } from "~/lib/utils";

// Collapsed-by-default "Archived (n)" section per entity list. Kept out of the
// active flow so archived rows never crowd the working set, while restore /
// permanent-delete stay one expand away.
export function ArchivedSection({
  count,
  children,
  className,
}: {
  count: number;
  children: React.ReactNode;
  className?: string;
}) {
  const [open, setOpen] = React.useState(false);
  if (count === 0) return null;
  return (
    <div className={cn("flex flex-col", className)} data-testid="archived-section">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        aria-expanded={open}
        data-testid="archived-toggle"
        className="flex items-center gap-1 px-3 py-0.5 text-[0.75rem] font-medium uppercase tracking-wider text-muted-foreground hover:text-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
      >
        {open ? (
          <ChevronDown strokeWidth={2} className="size-[var(--icon-sm)]" />
        ) : (
          <ChevronRight strokeWidth={2} className="size-[var(--icon-sm)]" />
        )}
        <span>Archived ({count})</span>
      </button>
      {open && <div className="flex flex-col gap-px">{children}</div>}
    </div>
  );
}

// One archived row: name plus restore and permanent-delete affordances. The host
// owns both mutations (restore returns it to the active list; delete confirms).
export function ArchivedRow({
  name,
  onRestore,
  onDelete,
}: {
  name: string;
  onRestore: () => void;
  onDelete: () => void;
}) {
  return (
    <div
      data-testid="archived-row"
      className="group flex items-center gap-1 px-3 h-7 rounded-sm hover:bg-muted/50"
    >
      <span className="flex-1 truncate text-[0.833rem] text-muted-foreground line-through">
        {name}
      </span>
      <Tooltip>
        <TooltipTrigger
          type="button"
          onClick={onRestore}
          aria-label={`Restore ${name}`}
          className="opacity-0 group-hover:opacity-100 focus-visible:opacity-100 flex items-center p-0.5 rounded-sm text-muted-foreground hover:text-foreground hover:bg-muted focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
        >
          <ArchiveRestore strokeWidth={2} className="size-[var(--icon-sm)]" />
        </TooltipTrigger>
        <TooltipContent>Restore {name}</TooltipContent>
      </Tooltip>
      <Tooltip>
        <TooltipTrigger
          type="button"
          onClick={onDelete}
          aria-label={`Delete ${name}`}
          className="opacity-0 group-hover:opacity-100 focus-visible:opacity-100 flex items-center p-0.5 rounded-sm text-muted-foreground hover:text-destructive hover:bg-muted focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
        >
          <Trash2 strokeWidth={2} className="size-[var(--icon-sm)]" />
        </TooltipTrigger>
        <TooltipContent>Delete {name}</TooltipContent>
      </Tooltip>
    </div>
  );
}
