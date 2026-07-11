import type { CommandView } from "@tillerd/client-bindings";

import { Plus, X } from "lucide-react";
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
import { Tooltip, TooltipContent, TooltipTrigger } from "~/components/ui/tooltip";

const REMOVE_ROW_CLASS =
  "flex items-center justify-center w-7 h-7 shrink-0 rounded-sm text-muted-foreground hover:text-destructive hover:bg-destructive/10 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring";

export interface CommandFormValues {
  name: string;
  cli: string;
  args: string[];
  env: Record<string, string>;
}

function envToRows(env: Record<string, string>): { key: string; value: string }[] {
  return Object.entries(env).map(([key, value]) => ({ key, value }));
}

function rowsToEnv(rows: { key: string; value: string }[]): Record<string, string> {
  const env: Record<string, string> = {};
  for (const row of rows) {
    if (row.key.trim()) env[row.key.trim()] = row.value;
  }
  return env;
}

// Create/edit form for a library command. `command` null means create; present
// means edit, pre-filled from the row's already-loaded CommandView (no extra
// fetch). Args/env belong to the command itself, not a launch item (see
// launchSpec.ts) -- this is the one place they're edited.
export function CommandFormDialog({
  open,
  command,
  submitError,
  onOpenChange,
  onSubmit,
}: {
  open: boolean;
  command: CommandView | null;
  submitError: string | null;
  onOpenChange: (open: boolean) => void;
  onSubmit: (values: CommandFormValues) => void;
}) {
  const [name, setName] = React.useState("");
  const [cli, setCli] = React.useState("");
  const [args, setArgs] = React.useState<string[]>([]);
  const [envRows, setEnvRows] = React.useState<{ key: string; value: string }[]>([]);
  const [nameError, setNameError] = React.useState(false);
  const [cliError, setCliError] = React.useState(false);

  React.useEffect(() => {
    if (!open) return;
    setName(command?.name ?? "");
    setCli(command?.cli ?? "");
    setArgs(command?.args ?? []);
    setEnvRows(command ? envToRows(command.env) : []);
    setNameError(false);
    setCliError(false);
  }, [open, command]);

  const handleSubmit = () => {
    const trimmedName = name.trim();
    const trimmedCli = cli.trim();
    const nameInvalid = trimmedName.length === 0;
    const cliInvalid = trimmedCli.length === 0;
    setNameError(nameInvalid);
    setCliError(cliInvalid);
    if (nameInvalid || cliInvalid) return;
    onSubmit({
      name: trimmedName,
      cli: trimmedCli,
      args: args.filter((a) => a.trim().length > 0),
      env: rowsToEnv(envRows),
    });
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent data-testid="command-form-dialog" className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{command ? "Edit command" : "New command"}</DialogTitle>
        </DialogHeader>

        <div className="flex flex-col gap-3">
          <div className="flex flex-col gap-1">
            <Label htmlFor="command-form-name">Name</Label>
            <Input
              id="command-form-name"
              data-testid="command-form-name"
              value={name}
              aria-invalid={nameError}
              onChange={(e) => {
                setName(e.target.value);
                if (nameError && e.target.value.trim()) setNameError(false);
              }}
            />
            {nameError && (
              <p className="text-destructive text-[0.75rem]" data-testid="command-form-name-error">
                Name is required
              </p>
            )}
          </div>

          <div className="flex flex-col gap-1">
            <Label htmlFor="command-form-cli">CLI</Label>
            <Input
              id="command-form-cli"
              data-testid="command-form-cli"
              value={cli}
              aria-invalid={cliError}
              onChange={(e) => {
                setCli(e.target.value);
                if (cliError && e.target.value.trim()) setCliError(false);
              }}
            />
            {cliError && (
              <p className="text-destructive text-[0.75rem]" data-testid="command-form-cli-error">
                CLI is required
              </p>
            )}
          </div>

          <div className="flex flex-col gap-1.5">
            <div className="flex items-center justify-between">
              <Label>Arguments</Label>
              <Button
                type="button"
                variant="ghost"
                size="xs"
                aria-label="Add argument"
                data-testid="command-form-add-arg"
                onClick={() => setArgs((prev) => [...prev, ""])}
              >
                <Plus className="size-[var(--icon-sm)]" />
                Add
              </Button>
            </div>
            {args.map((arg, i) => (
              <div key={i} className="flex items-center gap-1">
                <Input
                  aria-label={`Argument ${i + 1}`}
                  data-testid="command-form-arg"
                  value={arg}
                  onChange={(e) =>
                    setArgs((prev) => prev.map((a, idx) => (idx === i ? e.target.value : a)))
                  }
                />
                <Tooltip>
                  <TooltipTrigger
                    aria-label={`Remove argument ${i + 1}`}
                    onClick={() => setArgs((prev) => prev.filter((_, idx) => idx !== i))}
                    className={REMOVE_ROW_CLASS}
                  >
                    <X className="size-[var(--icon-sm)]" />
                  </TooltipTrigger>
                  <TooltipContent>Remove argument</TooltipContent>
                </Tooltip>
              </div>
            ))}
          </div>

          <div className="flex flex-col gap-1.5">
            <div className="flex items-center justify-between">
              <Label>Environment</Label>
              <Button
                type="button"
                variant="ghost"
                size="xs"
                aria-label="Add environment variable"
                data-testid="command-form-add-env"
                onClick={() => setEnvRows((prev) => [...prev, { key: "", value: "" }])}
              >
                <Plus className="size-[var(--icon-sm)]" />
                Add
              </Button>
            </div>
            {envRows.map((row, i) => (
              <div key={i} className="flex items-center gap-1">
                <Input
                  aria-label={`Environment key ${i + 1}`}
                  data-testid="command-form-env-key"
                  placeholder="KEY"
                  value={row.key}
                  onChange={(e) =>
                    setEnvRows((prev) =>
                      prev.map((r, idx) => (idx === i ? { ...r, key: e.target.value } : r)),
                    )
                  }
                />
                <Input
                  aria-label={`Environment value ${i + 1}`}
                  data-testid="command-form-env-value"
                  placeholder="value"
                  value={row.value}
                  onChange={(e) =>
                    setEnvRows((prev) =>
                      prev.map((r, idx) => (idx === i ? { ...r, value: e.target.value } : r)),
                    )
                  }
                />
                <Tooltip>
                  <TooltipTrigger
                    aria-label={`Remove environment variable ${i + 1}`}
                    onClick={() => setEnvRows((prev) => prev.filter((_, idx) => idx !== i))}
                    className={REMOVE_ROW_CLASS}
                  >
                    <X className="size-[var(--icon-sm)]" />
                  </TooltipTrigger>
                  <TooltipContent>Remove environment variable</TooltipContent>
                </Tooltip>
              </div>
            ))}
          </div>

          {submitError && (
            <p className="text-destructive text-[0.833rem]" data-testid="command-form-error">
              {submitError}
            </p>
          )}
        </div>

        <DialogFooter>
          <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button type="button" onClick={handleSubmit} data-testid="command-form-save">
            Save
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
