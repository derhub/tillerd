import type { CommandView } from "@tillerd/client-bindings";

import { useQuery } from "@tanstack/react-query";
import { ArrowDown, ArrowUp, Plus, X } from "lucide-react";
import React from "react";

import { Button } from "~/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "~/components/ui/dialog";
import { Input } from "~/components/ui/input";
import { Tooltip, TooltipContent, TooltipTrigger } from "~/components/ui/tooltip";
import { commandListQuery } from "~/lib/data/commands";
import {
  isLibraryRef,
  newLibraryItem,
  validateSpec,
  type LaunchItem,
  type LaunchSpec,
} from "~/lib/launchSpec";
import { cn } from "~/lib/utils";

const ITEM_ACTION_CLASS =
  "flex items-center justify-center w-6 h-6 rounded-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring";

function moveItem<T>(items: T[], from: number, to: number): T[] {
  const next = items.slice();
  const [item] = next.splice(from, 1);
  next.splice(to, 0, item);
  return next;
}

// One launch item's editor row: a library command reference (default) or an
// inline executable + space-separated arguments (the alternate mode, preserved
// verbatim for an item the editor didn't create -- see launchSpec.ts). Plain
// native <select> rather than the shadcn Select: this dialog's list is edited
// with keyboard/fireEvent in unit tests and a native control needs no pointer
// choreography to change value.
function ItemEditor({
  item,
  index,
  count,
  commands,
  onChange,
  onRemove,
  onMoveUp,
  onMoveDown,
}: {
  item: LaunchItem;
  index: number;
  count: number;
  commands: CommandView[];
  onChange: (next: LaunchItem) => void;
  onRemove: () => void;
  onMoveUp: () => void;
  onMoveDown: () => void;
}) {
  const isLibrary = isLibraryRef(item.command);

  return (
    <div
      className="flex flex-col gap-2 rounded-sm border border-border/60 p-2"
      data-testid="spec-item"
    >
      <div className="flex items-center gap-2">
        <span className="text-[0.75rem] text-muted-foreground/60 shrink-0">Item {index + 1}</span>
        <div className="flex-1" />
        <Tooltip>
          {/* `disabled` on TooltipTrigger only suppresses the tooltip popup, not the
              rendered element -- pass it through `render` so the button itself is
              actually disabled (native semantics + the disabled: variant below). */}
          <TooltipTrigger
            aria-label="Move up"
            disabled={index === 0}
            onClick={onMoveUp}
            render={<button type="button" disabled={index === 0} />}
            className={cn(ITEM_ACTION_CLASS, "text-muted-foreground hover:text-foreground hover:bg-muted disabled:opacity-30")}
          >
            <ArrowUp className="size-[var(--icon-sm)]" />
          </TooltipTrigger>
          <TooltipContent>Move up</TooltipContent>
        </Tooltip>
        <Tooltip>
          <TooltipTrigger
            aria-label="Move down"
            disabled={index === count - 1}
            onClick={onMoveDown}
            render={<button type="button" disabled={index === count - 1} />}
            className={cn(ITEM_ACTION_CLASS, "text-muted-foreground hover:text-foreground hover:bg-muted disabled:opacity-30")}
          >
            <ArrowDown className="size-[var(--icon-sm)]" />
          </TooltipTrigger>
          <TooltipContent>Move down</TooltipContent>
        </Tooltip>
        <Tooltip>
          <TooltipTrigger
            aria-label={`Remove item ${index + 1}`}
            onClick={onRemove}
            className={cn(ITEM_ACTION_CLASS, "text-muted-foreground hover:text-destructive hover:bg-destructive/10")}
          >
            <X className="size-[var(--icon-sm)]" />
          </TooltipTrigger>
          <TooltipContent>Remove item</TooltipContent>
        </Tooltip>
      </div>

      <div className="flex items-center gap-1">
        <button
          type="button"
          aria-pressed={isLibrary}
          onClick={() => onChange({ ...item, command: { library_ref: "" } })}
          className={cn(
            "h-6 px-2 rounded-sm text-[0.75rem] focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
            isLibrary ? "bg-muted text-foreground" : "text-muted-foreground hover:bg-muted/50",
          )}
        >
          Library command
        </button>
        <button
          type="button"
          aria-pressed={!isLibrary}
          onClick={() => onChange({ ...item, command: { executable: "", args: [] } })}
          className={cn(
            "h-6 px-2 rounded-sm text-[0.75rem] focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
            !isLibrary ? "bg-muted text-foreground" : "text-muted-foreground hover:bg-muted/50",
          )}
        >
          Inline executable
        </button>
      </div>

      {isLibrary ? (
        <select
          aria-label={`Command for item ${index + 1}`}
          data-testid="spec-item-command"
          value={isLibraryRef(item.command) ? item.command.library_ref : ""}
          onChange={(e) => onChange({ ...item, command: { library_ref: e.target.value } })}
          className="h-8 rounded-lg border border-input bg-transparent px-2.5 text-[0.833rem] focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
        >
          <option value="">Select a command…</option>
          {commands.map((c) => (
            <option key={c.id} value={c.id}>
              {c.name}
            </option>
          ))}
        </select>
      ) : (
        <div className="flex flex-col gap-1.5">
          <Input
            aria-label={`Executable for item ${index + 1}`}
            data-testid="spec-item-executable"
            placeholder="Executable"
            value={!isLibraryRef(item.command) ? item.command.executable : ""}
            onChange={(e) =>
              onChange({
                ...item,
                command: {
                  executable: e.target.value,
                  args: !isLibraryRef(item.command) ? item.command.args : [],
                },
              })
            }
          />
          <Input
            aria-label={`Arguments for item ${index + 1}`}
            data-testid="spec-item-args"
            placeholder="Arguments (space-separated)"
            value={!isLibraryRef(item.command) ? item.command.args.join(" ") : ""}
            onChange={(e) =>
              onChange({
                ...item,
                command: {
                  executable: !isLibraryRef(item.command) ? item.command.executable : "",
                  args: e.target.value.split(/\s+/).filter((a) => a.length > 0),
                },
              })
            }
          />
        </div>
      )}

      <Input
        aria-label={`Placement for item ${index + 1}`}
        data-testid="spec-item-placement"
        placeholder="Placement (optional)"
        value={item.placement ?? ""}
        onChange={(e) => {
          const value = e.target.value;
          onChange(value ? { ...item, placement: value } : { target: item.target, command: item.command });
        }}
      />
    </div>
  );
}

// Visual form editor over a versioned launch spec (launchSpec.ts). Used both to
// create and edit a project launch template -- there is no library-template
// update op, so the library side only ever imports/exports raw spec JSON (see
// TemplatesView). Raw JSON is never exposed here (spec: "not required in this
// flow").
export function SpecEditorDialog({
  open,
  title,
  spec,
  saveError,
  onOpenChange,
  onSave,
}: {
  open: boolean;
  title: string;
  spec: LaunchSpec;
  saveError: string | null;
  onOpenChange: (open: boolean) => void;
  onSave: (spec: LaunchSpec) => void;
}) {
  const { data: commands = [] } = useQuery(commandListQuery());
  const [items, setItems] = React.useState<LaunchItem[]>(spec.items);
  const [errors, setErrors] = React.useState<string[]>([]);

  React.useEffect(() => {
    if (open) {
      setItems(spec.items);
      setErrors([]);
    }
  }, [open, spec]);

  const handleSave = () => {
    const next: LaunchSpec = { version: spec.version, items };
    const validationErrors = validateSpec(next);
    setErrors(validationErrors);
    if (validationErrors.length > 0) return;
    onSave(next);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent data-testid="spec-editor-dialog" className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
        </DialogHeader>

        <div className="flex flex-col gap-2 max-h-96 overflow-y-auto">
          {items.length === 0 && (
            <p className="text-[0.833rem] text-muted-foreground/60 italic">No items yet</p>
          )}
          {items.map((item, i) => (
            <ItemEditor
              key={i}
              item={item}
              index={i}
              count={items.length}
              commands={commands}
              onChange={(next) => setItems((prev) => prev.map((it, idx) => (idx === i ? next : it)))}
              onRemove={() => setItems((prev) => prev.filter((_, idx) => idx !== i))}
              onMoveUp={() => i > 0 && setItems((prev) => moveItem(prev, i, i - 1))}
              onMoveDown={() =>
                i < items.length - 1 && setItems((prev) => moveItem(prev, i, i + 1))
              }
            />
          ))}
          <Button
            type="button"
            variant="outline"
            size="sm"
            data-testid="spec-add-item"
            onClick={() => setItems((prev) => [...prev, newLibraryItem()])}
          >
            <Plus className="size-[var(--icon-md)]" />
            Add item
          </Button>
        </div>

        {errors.length > 0 && (
          <div className="flex flex-col gap-0.5" data-testid="spec-editor-errors">
            {errors.map((e) => (
              <p key={e} className="text-destructive text-[0.75rem]">
                {e}
              </p>
            ))}
          </div>
        )}
        {saveError && (
          <p className="text-destructive text-[0.833rem]" data-testid="spec-editor-save-error">
            {saveError}
          </p>
        )}

        <DialogFooter>
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button type="button" onClick={handleSave} data-testid="spec-editor-save">
            Save
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
