import { ArchiveRestore, ChevronDown, ChevronRight, Trash2 } from "lucide-react";
import React from "react";

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
        className="flex items-center gap-1 px-3 py-0.5 text-[0.75rem] uppercase tracking-wider text-muted-foreground/50 hover:text-foreground"
      >
        {open ? (
          <ChevronDown size={10} strokeWidth={2} />
        ) : (
          <ChevronRight size={10} strokeWidth={2} />
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
      <span className="flex-1 truncate text-[0.833rem] text-muted-foreground/60 line-through">
        {name}
      </span>
      <button
        type="button"
        onClick={onRestore}
        aria-label={`Restore ${name}`}
        title="Restore"
        className="opacity-0 group-hover:opacity-100 flex items-center p-0.5 rounded-sm text-muted-foreground/50 hover:text-foreground hover:bg-muted"
      >
        <ArchiveRestore size={10} strokeWidth={2} />
      </button>
      <button
        type="button"
        onClick={onDelete}
        aria-label={`Delete ${name}`}
        title="Delete permanently"
        className="opacity-0 group-hover:opacity-100 flex items-center p-0.5 rounded-sm text-muted-foreground/50 hover:text-destructive hover:bg-muted"
      >
        <Trash2 size={10} strokeWidth={2} />
      </button>
    </div>
  );
}
