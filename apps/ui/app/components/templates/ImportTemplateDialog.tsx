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
import { Label } from "~/components/ui/label";

export interface PendingImport {
  fileName: string;
  specVersion: number;
  specJson: string;
}

// Names an already-picked, already-parsed spec file before it's sent to
// `template_import` -- the exported file carries the spec only (spec: "supplying
// a name (the exported file carries the spec only)").
export function ImportTemplateDialog({
  pending,
  onCancel,
  onConfirm,
}: {
  pending: PendingImport | null;
  onCancel: () => void;
  onConfirm: (name: string) => void;
}) {
  const [name, setName] = React.useState("");
  const [error, setError] = React.useState(false);

  React.useEffect(() => {
    if (pending) {
      setName(pending.fileName.replace(/\.json$/i, ""));
      setError(false);
    }
  }, [pending]);

  if (!pending) return null;

  const handleConfirm = () => {
    const trimmed = name.trim();
    if (!trimmed) {
      setError(true);
      return;
    }
    onConfirm(trimmed);
  };

  return (
    <Dialog open onOpenChange={(open) => !open && onCancel()}>
      <DialogContent data-testid="template-import-dialog">
        <DialogHeader>
          <DialogTitle>Name this template</DialogTitle>
        </DialogHeader>
        <div className="flex flex-col gap-1">
          <Label htmlFor="template-import-name">Name</Label>
          <Input
            id="template-import-name"
            data-testid="template-import-name"
            value={name}
            aria-invalid={error}
            onChange={(e) => {
              setName(e.target.value);
              if (error && e.target.value.trim()) setError(false);
            }}
          />
          {error && <p className="text-destructive text-[0.75rem]">Name is required</p>}
        </div>
        <DialogFooter>
          <Button type="button" variant="outline" onClick={onCancel}>
            Cancel
          </Button>
          <Button type="button" data-testid="template-import-confirm" onClick={handleConfirm}>
            Import
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
