import type { ProfileView } from "@tillerd/client-bindings";

import { useMutation, useQuery } from "@tanstack/react-query";
import { command, query } from "@tillerd/client-bindings";
import { Copy, Download, Plus, Trash2, Upload } from "lucide-react";
import React from "react";

import { InlineRenameInput } from "~/components/sidebar/InlineRenameInput";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogTitle,
} from "~/components/ui/alert-dialog";
import { Button } from "~/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "~/components/ui/tooltip";
import { hydrateSettings } from "~/lib/settings/context";
import { cn } from "~/lib/utils";

export interface ProfilesListProps {
  profiles: ProfileView[];
  activeId: string | null;
  editingId: string | null;
  onActivate: (id: string) => void;
  onStartEdit: (id: string) => void;
  onCancelEdit: () => void;
  onRename: (id: string, name: string) => void;
  onDuplicate: (id: string) => void;
  onRequestDelete: (id: string, name: string) => void;
  onExport: (id: string) => void;
}

export function ProfilesList({
  profiles,
  activeId,
  editingId,
  onActivate,
  onStartEdit,
  onCancelEdit,
  onRename,
  onDuplicate,
  onRequestDelete,
  onExport,
}: ProfilesListProps) {
  if (profiles.length === 0) {
    return <p className="text-muted-foreground/60 italic text-[0.917rem]">No profiles</p>;
  }

  return (
    <ul className="flex flex-col gap-0.5" data-testid="profiles-list">
      {profiles.map((p) => (
        <li
          key={p.id}
          data-testid="profile-row"
          data-profile-id={p.id}
          className="flex items-center gap-2 h-7 px-1"
        >
          {editingId === p.id ? (
            <InlineRenameInput
              initialValue={p.name}
              onConfirm={(name) => onRename(p.id, name)}
              onCancel={onCancelEdit}
            />
          ) : (
            <button
              type="button"
              aria-current={p.id === activeId ? "true" : undefined}
              onDoubleClick={() => onStartEdit(p.id)}
              onClick={() => onActivate(p.id)}
              className={cn(
                "flex-1 text-left truncate text-[0.917rem] px-2 py-0.5 rounded-sm transition-colors duration-[var(--motion-fast)] ease-standard",
                "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
                p.id === activeId
                  ? "font-medium bg-muted text-foreground"
                  : "text-muted-foreground hover:text-foreground hover:bg-muted",
              )}
            >
              {p.name}
            </button>
          )}
          {p.id === activeId && (
            <span
              data-testid="profile-active-badge"
              className="text-[0.75rem] uppercase tracking-[0.05em] text-primary shrink-0"
            >
              Active
            </span>
          )}
          <Tooltip>
            <TooltipTrigger
              type="button"
              aria-label={`Duplicate ${p.name}`}
              onClick={() => onDuplicate(p.id)}
              className="flex items-center justify-center w-6 h-6 rounded-sm text-muted-foreground hover:text-foreground hover:bg-muted focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
            >
              <Copy className="size-[var(--icon-sm)]" />
            </TooltipTrigger>
            <TooltipContent>Duplicate</TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger
              type="button"
              aria-label={`Export ${p.name}`}
              onClick={() => onExport(p.id)}
              className="flex items-center justify-center w-6 h-6 rounded-sm text-muted-foreground hover:text-foreground hover:bg-muted focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
            >
              <Download className="size-[var(--icon-sm)]" />
            </TooltipTrigger>
            <TooltipContent>Export</TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger
              type="button"
              aria-label={`Delete ${p.name}`}
              onClick={() => onRequestDelete(p.id, p.name)}
              className="flex items-center justify-center w-6 h-6 rounded-sm text-muted-foreground hover:text-destructive hover:bg-destructive/10 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
            >
              <Trash2 className="size-[var(--icon-sm)]" />
            </TooltipTrigger>
            <TooltipContent>Delete</TooltipContent>
          </Tooltip>
        </li>
      ))}
    </ul>
  );
}

function downloadBytes(bytes: number[], filename: string): void {
  const blob = new Blob([new Uint8Array(bytes)], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}

// Plain async helper (not a hook/component) -- the no-async-in-component rule exempts these;
// components fire mutations via mutate(), never await mutateAsync() or .then() themselves.
function importProfileFile(
  file: File,
  importProfile: { mutateAsync: (args: { profileJson: string }) => Promise<unknown> },
  onError: (message: string) => void,
): void {
  void file
    .text()
    .then((profileJson) => importProfile.mutateAsync({ profileJson }))
    .catch(() => onError(`Could not import ${file.name}`));
}

export function ProfilesSection() {
  const { data: profiles } = useQuery(query("profileList"));
  const { data: active } = useQuery(query("profileGetActive"));
  const activeId = active?.id ?? null;

  const [editingId, setEditingId] = React.useState<string | null>(null);
  const [deleteTarget, setDeleteTarget] = React.useState<{ id: string; name: string } | null>(null);
  const [importError, setImportError] = React.useState<string | null>(null);
  const fileInputRef = React.useRef<HTMLInputElement>(null);

  const createProfile = useMutation(command("profileCreate"));
  const activateProfile = useMutation(command("profileActivate"));
  const renameProfile = useMutation(command("profileRename"));
  const duplicateProfile = useMutation(command("profileDuplicate"));
  const discardProfile = useMutation(command("profileDiscard"));
  const exportProfile = useMutation(command("profileExport"));
  const importProfile = useMutation(command("profileImport"));

  // Activating a profile swaps its resolved settings server-side; the local settings
  // store only converges via explicit re-hydration (its own writes are excluded from
  // the cross-window invalidation listener), so this refetches the store the same way
  // the provider bootstraps -- otherwise theme/scheme/keybindings would only pick up
  // the new profile after a reload.
  const rehydrate = React.useCallback(() => void hydrateSettings(), []);

  const handleCreate = React.useCallback(() => {
    const id = crypto.randomUUID();
    createProfile.mutate({ id, name: "New profile" }, { onSuccess: (p) => setEditingId(p.id) });
  }, [createProfile]);

  const handleDuplicate = React.useCallback(
    (sourceId: string) => {
      const source = profiles?.find((p) => p.id === sourceId);
      const newId = crypto.randomUUID();
      duplicateProfile.mutate(
        { sourceId, newId, newName: `${source?.name ?? "Profile"} copy` },
        { onSuccess: () => setEditingId(newId) },
      );
    },
    [profiles, duplicateProfile],
  );

  const handleConfirmDelete = React.useCallback(() => {
    if (!deleteTarget) return;
    const wasActive = deleteTarget.id === activeId;
    discardProfile.mutate(
      { id: deleteTarget.id },
      {
        onSuccess: () => {
          setDeleteTarget(null);
          if (wasActive) rehydrate();
        },
      },
    );
  }, [deleteTarget, activeId, discardProfile, rehydrate]);

  const handleExport = React.useCallback(
    (id: string) => {
      const p = profiles?.find((row) => row.id === id);
      exportProfile.mutate(
        { id },
        {
          onSuccess: (bytes) => {
            if (bytes) downloadBytes(bytes, `${p?.name ?? id}.profile.json`);
          },
        },
      );
    },
    [profiles, exportProfile],
  );

  const handleImportFile = React.useCallback(
    (file: File) => {
      setImportError(null);
      importProfileFile(file, importProfile, setImportError);
    },
    [importProfile],
  );

  return (
    <section aria-labelledby="settings-profiles-heading" className="flex flex-col gap-3 max-w-md">
      <div className="flex items-center justify-between gap-3">
        <h2
          id="settings-profiles-heading"
          className="text-[0.75rem] font-medium uppercase tracking-[0.05em] text-muted-foreground"
        >
          Profiles
        </h2>
        <div className="flex items-center gap-1">
          <input
            ref={fileInputRef}
            type="file"
            accept="application/json"
            className="hidden"
            onChange={(e) => {
              const file = e.target.files?.[0];
              if (file) handleImportFile(file);
              e.target.value = "";
            }}
          />
          <Button variant="ghost" size="xs" onClick={() => fileInputRef.current?.click()}>
            <Upload className="size-[var(--icon-sm)]" />
            Import
          </Button>
          <Button variant="outline" size="xs" onClick={handleCreate}>
            <Plus className="size-[var(--icon-sm)]" />
            New profile
          </Button>
        </div>
      </div>

      {importError && <p className="text-destructive text-[0.833rem]">{importError}</p>}

      <ProfilesList
        profiles={profiles ?? []}
        activeId={activeId}
        editingId={editingId}
        onActivate={(id) => activateProfile.mutate({ id }, { onSuccess: rehydrate })}
        onStartEdit={setEditingId}
        onCancelEdit={() => setEditingId(null)}
        onRename={(id, name) =>
          renameProfile.mutate({ id, newName: name }, { onSuccess: () => setEditingId(null) })
        }
        onDuplicate={handleDuplicate}
        onRequestDelete={(id, name) => setDeleteTarget({ id, name })}
        onExport={handleExport}
      />

      <AlertDialog
        open={deleteTarget != null}
        onOpenChange={(open) => !open && setDeleteTarget(null)}
      >
        <AlertDialogContent>
          <AlertDialogTitle>Delete {deleteTarget?.name}?</AlertDialogTitle>
          <AlertDialogDescription>
            {deleteTarget?.id === activeId
              ? "This profile is active. Deleting it switches to another profile immediately."
              : "This will permanently delete the profile."}
          </AlertDialogDescription>
          <div className="flex gap-2 justify-end">
            <AlertDialogCancel onClick={() => setDeleteTarget(null)}>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={handleConfirmDelete}
              className="bg-destructive hover:bg-destructive/90"
            >
              Delete
            </AlertDialogAction>
          </div>
        </AlertDialogContent>
      </AlertDialog>
    </section>
  );
}
