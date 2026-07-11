import React from "react";

import { Button } from "~/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "~/components/ui/dialog";
import { Input } from "~/components/ui/input";
import { Label } from "~/components/ui/label";

// `template_export` writes the raw spec JSON to a caller-supplied filesystem
// path -- there is no Tauri save-file dialog wired in this change, so the
// destination is a plain text field (see design decisions).
export function ExportTemplateDialog({
  target,
  onCancel,
  onExport,
}: {
  target: { id: string; name: string } | null;
  onCancel: () => void;
  onExport: (id: string, destPath: string) => void;
}) {
  const [destPath, setDestPath] = React.useState("");

  React.useEffect(() => {
    if (target) setDestPath("");
  }, [target]);

  if (!target) return null;

  return (
    <Dialog open onOpenChange={(open) => !open && onCancel()}>
      <DialogContent data-testid="template-export-dialog">
        <DialogHeader>
          <DialogTitle>Export {target.name}</DialogTitle>
        </DialogHeader>
        <DialogDescription>Choose a destination file path for the spec JSON.</DialogDescription>
        <div className="flex flex-col gap-1">
          <Label htmlFor="template-export-path">Destination path</Label>
          <Input
            id="template-export-path"
            data-testid="template-export-path"
            placeholder="/path/to/template.json"
            value={destPath}
            onChange={(e) => setDestPath(e.target.value)}
          />
        </div>
        <DialogFooter>
          <Button type="button" variant="outline" onClick={onCancel}>
            Cancel
          </Button>
          <Button
            type="button"
            data-testid="template-export-confirm"
            disabled={destPath.trim().length === 0}
            onClick={() => onExport(target.id, destPath.trim())}
          >
            Export
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
