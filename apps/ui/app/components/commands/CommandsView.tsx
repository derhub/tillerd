import type { CommandView as CommandRecord } from "@tillerd/client-bindings";

import { useMutation, useQuery } from "@tanstack/react-query";
import { command } from "@tillerd/client-bindings";
import { Plus } from "lucide-react";
import React from "react";

import { CommandFormDialog, type CommandFormValues } from "~/components/commands/CommandFormDialog";
import { CommandRow } from "~/components/commands/CommandRow";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogTitle,
} from "~/components/ui/alert-dialog";
import { Button } from "~/components/ui/button";
import { ScrollArea } from "~/components/ui/scroll-area";
import { ACTION } from "~/lib/commands/ids";
import { type CommandArgs, useDispatchCommand, useRegisterHandlers } from "~/lib/commands/registry";
import { commandListQuery } from "~/lib/data/commands";
import { notify } from "~/lib/notifications/notify";
import { SessionContext } from "~/lib/sessionContext";
import { useDesktopHost } from "~/lib/useDesktopHost";

const PAGE_SIZE = 50;

// Commands activity-bar view: the command library (prebuilt + custom), pinned
// first (command_list already orders pinned DESC server-side). The list is
// unpaginated on the wire, so this windows it client-side rather than assume a
// small fixed set (spec: list SHALL paginate or virtualize).
export function CommandsView() {
  const isDesktop = useDesktopHost().status === "ready";
  const { sessionId } = React.use(SessionContext);
  const { data: commands = [] } = useQuery(commandListQuery());

  const [visibleCount, setVisibleCount] = React.useState(PAGE_SIZE);
  const [renamingId, setRenamingId] = React.useState<string | null>(null);
  const [formTarget, setFormTarget] = React.useState<CommandRecord | null | "create">(null);
  const [formError, setFormError] = React.useState<string | null>(null);
  const [deleteTarget, setDeleteTarget] = React.useState<{ id: string; name: string } | null>(null);

  const createCommand = useMutation(command("commandCreate"));
  const editCommand = useMutation(command("commandEdit"));
  const renameCommand = useMutation(command("commandRename"));
  const duplicateCommand = useMutation(command("commandDuplicate"));
  const pinCommand = useMutation(command("commandPin"));
  const unpinCommand = useMutation(command("commandUnpin"));
  const deleteCommand = useMutation(command("commandDelete"));
  const dispatch = useDispatchCommand();

  const visible = commands.slice(0, visibleCount);

  const handleStartRename = React.useCallback((id: string) => setRenamingId(id), []);
  const handleCancelRename = React.useCallback(() => setRenamingId(null), []);
  const handleConfirmRename = React.useCallback(
    (id: string, name: string) => {
      const trimmed = name.trim();
      if (!trimmed) return;
      renameCommand.mutate({ id, name: trimmed }, { onSuccess: () => setRenamingId(null) });
    },
    [renameCommand],
  );

  const handleDuplicate = React.useCallback(
    (id: string) => {
      const source = commands.find((c) => c.id === id);
      if (!source) return;
      duplicateCommand.mutate({ id, name: `${source.name} copy` });
    },
    [commands, duplicateCommand],
  );

  // Run row action (usability pass 12.6): dispatches the spawn to PanelContent (the panel-tree
  // owner) so the command's PTY is placed into a leaf and rendered -- this view is out of the
  // tree and cannot place surfaces itself. No active session is a real, reachable state (e.g.
  // commands view open with no session selected) -- surfaced via the notification center rather
  // than silently doing nothing.
  const handleRun = React.useCallback(
    (id: string) => {
      if (!sessionId) {
        notify("command-run", "error", "Open a session before running a command.");
        return;
      }
      dispatch(ACTION.surfaceRunCommand, { commandRef: { libraryRef: id } });
    },
    [sessionId, dispatch],
  );

  const handleRequestDelete = React.useCallback(
    (id: string, name: string) => setDeleteTarget({ id, name }),
    [],
  );

  const handleConfirmDelete = React.useCallback(() => {
    if (!deleteTarget) return;
    deleteCommand.mutate({ id: deleteTarget.id }, { onSuccess: () => setDeleteTarget(null) });
  }, [deleteTarget, deleteCommand]);

  const handleFormSubmit = React.useCallback(
    (values: CommandFormValues) => {
      setFormError(null);
      const onError = (e: Error) => setFormError(e.message);
      if (formTarget === "create") {
        createCommand.mutate(
          { req: { name: values.name, cli: values.cli, args: values.args, env: values.env } },
          { onSuccess: () => setFormTarget(null), onError },
        );
        return;
      }
      if (!formTarget) return;
      const id = formTarget.id;
      const nameChanged = values.name !== formTarget.name;
      const applyEdit = () =>
        editCommand.mutate(
          { id, cli: values.cli, args: values.args, env: values.env },
          { onSuccess: () => setFormTarget(null), onError },
        );
      if (nameChanged) {
        renameCommand.mutate({ id, name: values.name }, { onSuccess: applyEdit, onError });
      } else {
        applyEdit();
      }
    },
    [formTarget, createCommand, editCommand, renameCommand],
  );

  const registeredHandlers = React.useMemo(
    () => ({
      [ACTION.commandRun]: (args?: CommandArgs) => {
        if (args?.entityId) handleRun(args.entityId);
      },
      [ACTION.commandEdit]: (args?: CommandArgs) => {
        const target = commands.find((c) => c.id === args?.entityId);
        if (target) {
          setFormError(null);
          setFormTarget(target);
        }
      },
      [ACTION.commandRename]: (args?: CommandArgs) => {
        if (args?.entityId) handleStartRename(args.entityId);
      },
      [ACTION.commandDuplicate]: (args?: CommandArgs) => {
        if (args?.entityId) handleDuplicate(args.entityId);
      },
      [ACTION.commandPin]: (args?: CommandArgs) => {
        if (args?.entityId) pinCommand.mutate({ id: args.entityId });
      },
      [ACTION.commandUnpin]: (args?: CommandArgs) => {
        if (args?.entityId) unpinCommand.mutate({ id: args.entityId });
      },
      [ACTION.commandDelete]: (args?: CommandArgs) => {
        if (!args?.entityId) return;
        const label = typeof args.label === "string" ? args.label : "";
        handleRequestDelete(args.entityId, label);
      },
    }),
    [
      commands,
      handleStartRename,
      handleRun,
      handleDuplicate,
      pinCommand,
      unpinCommand,
      handleRequestDelete,
    ],
  );
  useRegisterHandlers(registeredHandlers);

  return (
    <div className="flex h-full flex-col">
      <div className="flex h-8 shrink-0 items-center justify-between px-3">
        <span className="text-[0.75rem] font-medium uppercase tracking-[0.05em] text-muted-foreground">
          Commands
        </span>
        {isDesktop && (
          <Button
            variant="ghost"
            size="xs"
            data-testid="command-create-button"
            onClick={() => {
              setFormError(null);
              setFormTarget("create");
            }}
          >
            <Plus className="size-[var(--icon-sm)]" />
            New
          </Button>
        )}
      </div>

      <ScrollArea className="flex-1 min-h-0">
        {commands.length === 0 ? (
          <p
            data-testid="commands-empty"
            className="px-3 py-3 text-[0.833rem] text-muted-foreground italic"
          >
            No commands yet
          </p>
        ) : (
          <div className="flex flex-col gap-px py-1" data-testid="commands-list">
            {visible.map((c) => (
              <CommandRow
                key={c.id}
                command={c}
                isDesktop={isDesktop}
                isRenaming={renamingId === c.id}
                onStartRename={handleStartRename}
                onConfirmRename={(name) => handleConfirmRename(c.id, name)}
                onCancelRename={handleCancelRename}
                onRun={handleRun}
                onEdit={(id) => {
                  const target = commands.find((row) => row.id === id);
                  if (target) {
                    setFormError(null);
                    setFormTarget(target);
                  }
                }}
                onDuplicate={handleDuplicate}
                onPin={(id) => pinCommand.mutate({ id })}
                onUnpin={(id) => unpinCommand.mutate({ id })}
                onRequestDelete={handleRequestDelete}
              />
            ))}
            {commands.length > visibleCount && (
              <button
                type="button"
                data-testid="commands-show-more"
                onClick={() => setVisibleCount((n) => n + PAGE_SIZE)}
                className="text-left text-[0.833rem] text-muted-foreground hover:text-foreground px-3 h-7 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
              >
                Show more ({commands.length - visibleCount} remaining)
              </button>
            )}
          </div>
        )}
      </ScrollArea>

      <CommandFormDialog
        open={formTarget !== null}
        command={formTarget === "create" ? null : formTarget}
        submitError={formError}
        onOpenChange={(open) => {
          if (!open) {
            setFormTarget(null);
            setFormError(null);
          }
        }}
        onSubmit={handleFormSubmit}
      />

      <AlertDialog
        open={deleteTarget != null}
        onOpenChange={(open) => !open && setDeleteTarget(null)}
      >
        <AlertDialogContent data-testid="command-delete-confirm">
          <AlertDialogTitle>Delete {deleteTarget?.name}?</AlertDialogTitle>
          <AlertDialogDescription>
            This will permanently delete the command from your library.
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
    </div>
  );
}
