import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogTitle,
} from "~/components/ui/alert-dialog";
import { Dialog, DialogContent, DialogDescription, DialogTitle } from "~/components/ui/dialog";
import { cn } from "~/lib/utils";

// Move a project to another workspace / a session to another project. The target
// list is supplied by the hub (valid, active, current excluded); selecting a row
// fires the move and closes.
export interface MoveTarget {
  id: string;
  name: string;
  kind: "project" | "session";
  targets: { id: string; name: string }[];
}

const MOVE_LABEL: Record<MoveTarget["kind"], { title: string; noun: string }> = {
  project: { title: "Move to workspace", noun: "workspace" },
  session: { title: "Move to project", noun: "project" },
};

export function MovePickerDialog({
  target,
  onCancel,
  onPick,
}: {
  target: MoveTarget | null;
  onCancel: () => void;
  onPick: (targetId: string) => void;
}) {
  if (!target) return null;
  const { title, noun } = MOVE_LABEL[target.kind];
  return (
    <Dialog open onOpenChange={(open) => !open && onCancel()}>
      <DialogContent data-testid="move-picker">
        <DialogTitle>{title}</DialogTitle>
        <DialogDescription>
          Move {target.name} to another {noun}.
        </DialogDescription>
        <div className="flex flex-col gap-px max-h-64 overflow-y-auto">
          {target.targets.length === 0 ? (
            <p className="px-2 py-2 text-[0.833rem] text-muted-foreground italic">
              No other {noun}s available
            </p>
          ) : (
            target.targets.map((t) => (
              <button
                key={t.id}
                type="button"
                data-testid="move-target"
                data-target-id={t.id}
                onClick={() => onPick(t.id)}
                className={cn(
                  "text-left text-[0.833rem] px-2 h-7 rounded-sm truncate transition-colors duration-[var(--motion-fast)] ease-standard",
                  "text-muted-foreground hover:text-foreground hover:bg-muted",
                )}
              >
                {t.name}
              </button>
            ))
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}

// Confirm stopping every running surface under a scope (workspace / project /
// session). The dialog names the scope; the hub owns the mutation.
export interface StopSurfacesTarget {
  id: string;
  name: string;
  kind: "workspace" | "project" | "session";
}

export function StopSurfacesDialog({
  target,
  onCancel,
  onConfirm,
}: {
  target: StopSurfacesTarget | null;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  if (!target) return null;
  return (
    <AlertDialog open onOpenChange={(open) => !open && onCancel()}>
      <AlertDialogContent data-testid="stop-surfaces-confirm">
        <AlertDialogTitle>Stop surfaces?</AlertDialogTitle>
        <AlertDialogDescription>
          This stops every running surface under the {target.kind} {target.name}. Sessions stay;
          their terminals end.
        </AlertDialogDescription>
        <div className="flex gap-2 justify-end">
          <AlertDialogCancel onClick={onCancel}>Cancel</AlertDialogCancel>
          <AlertDialogAction onClick={onConfirm}>Stop surfaces</AlertDialogAction>
        </div>
      </AlertDialogContent>
    </AlertDialog>
  );
}
