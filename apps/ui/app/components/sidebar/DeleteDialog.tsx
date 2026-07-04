import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogTitle,
} from "~/components/ui/alert-dialog";

export type DeleteTarget = { id: string; name: string; kind: "project" | "session" | "workspace" };

const DELETE_COPY: Record<DeleteTarget["kind"], string> = {
  project: "This will permanently delete the project and all its sessions.",
  session: "This will permanently delete the session and terminate its PTYs.",
  workspace: "This will permanently delete the workspace and all its projects and sessions.",
};

export function DeleteDialog({
  target,
  onCancel,
  onConfirm,
}: {
  target: DeleteTarget | null;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  if (!target) return null;
  return (
    <AlertDialog open={true} onOpenChange={(open) => !open && onCancel()}>
      <AlertDialogContent>
        <AlertDialogTitle>Delete {target.name}?</AlertDialogTitle>
        <AlertDialogDescription>{DELETE_COPY[target.kind]}</AlertDialogDescription>
        <div className="flex gap-2 justify-end">
          <AlertDialogCancel onClick={onCancel}>Cancel</AlertDialogCancel>
          <AlertDialogAction onClick={onConfirm} className="bg-destructive hover:bg-destructive/90">
            Delete
          </AlertDialogAction>
        </div>
      </AlertDialogContent>
    </AlertDialog>
  );
}
