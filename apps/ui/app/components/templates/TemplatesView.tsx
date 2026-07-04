import type { CommandView, LaunchTemplateView, TemplateView } from "@tillerd/client-bindings";

import { useMutation, useQuery } from "@tanstack/react-query";
import { command } from "@tillerd/client-bindings";
import { Plus, Upload } from "lucide-react";
import React from "react";

import { ExportTemplateDialog } from "~/components/templates/ExportTemplateDialog";
import {
  ImportTemplateDialog,
  type PendingImport,
} from "~/components/templates/ImportTemplateDialog";
import { LaunchTemplateRow } from "~/components/templates/LaunchTemplateRow";
import { SpecEditorDialog } from "~/components/templates/SpecEditorDialog";
import { TemplateRow } from "~/components/templates/TemplateRow";
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
import { type CommandArgs, useRegisterHandlers } from "~/lib/commands/registry";
import { commandListQuery } from "~/lib/data/commands";
import { launchTemplateListQuery, templateListQuery } from "~/lib/data/templates";
import {
  emptySpec,
  isLibraryRef,
  parseLaunchSpec,
  serializeLaunchSpec,
  type LaunchSpec,
} from "~/lib/launchSpec";
import { recordNotification } from "~/lib/notifications/context";
import { useActiveProject } from "~/lib/store";
import { useDesktopHost } from "~/lib/useDesktopHost";

function notify(category: string, severity: "info" | "error", message: string): void {
  recordNotification({
    id: crypto.randomUUID(),
    category,
    severity,
    title: null,
    message,
    detail: null,
    ts: Date.now(),
    sessionId: null,
    surfaceId: null,
  });
}

function launchTemplateLabel(
  view: LaunchTemplateView,
  commandsById: Map<string, CommandView>,
): string {
  let spec: LaunchSpec;
  try {
    spec = parseLaunchSpec(view.specJson);
  } catch {
    return "(invalid spec)";
  }
  const first = spec.items[0];
  if (!first) return "(empty template)";
  if (isLibraryRef(first.command)) {
    return commandsById.get(first.command.library_ref)?.name ?? "(unknown command)";
  }
  return first.command.executable || "(empty template)";
}

// Editor target: a new project launch template (no id yet) or an existing one
// being edited (id + its parsed spec). The library side never opens this editor
// -- there is no library-template update op (see design decisions).
type EditorTarget = { kind: "create" } | { kind: "edit"; id: string; spec: LaunchSpec };

// Plain async helper (not a hook/component) -- the no-async-in-component rule exempts these;
// components fire mutations via mutate(), never await/.then() themselves.
function readSpecFile(
  file: File,
  onParsed: (pending: PendingImport) => void,
  onError: (message: string) => void,
): void {
  void file.text().then((text) => {
    try {
      const spec = parseLaunchSpec(text);
      onParsed({ fileName: file.name, specVersion: spec.version, specJson: text });
    } catch {
      onError(`${file.name} is not a valid launch spec`);
    }
  });
}

// Templates activity-bar view: the portable library (prebuilt + custom) plus,
// when a project is active, that project's launch templates (spec:
// "Library and project sections").
export function TemplatesView() {
  const isDesktop = useDesktopHost().status === "ready";
  const activeProjectId = useActiveProject();

  const { data: templates = [] } = useQuery(templateListQuery());
  const { data: launchTemplates = [] } = useQuery({
    ...launchTemplateListQuery(activeProjectId ?? ""),
    enabled: Boolean(activeProjectId),
  });
  const { data: commands = [] } = useQuery(commandListQuery());
  const commandsById = React.useMemo(() => new Map(commands.map((c) => [c.id, c])), [commands]);

  const pinTemplate = useMutation(command("templatePin"));
  const unpinTemplate = useMutation(command("templateUnpin"));
  const discardTemplate = useMutation(command("templateDiscard"));
  const exportTemplate = useMutation(command("templateExport"));
  const importTemplate = useMutation(command("templateImport"));
  const createLaunchTemplate = useMutation(command("launchTemplateCreate"));
  const applyLaunchSpec = useMutation(command("launchTemplateApplySpec"));
  const discardLaunchTemplate = useMutation(command("launchTemplateDiscard"));

  const [deleteTarget, setDeleteTarget] = React.useState<{ id: string; name: string } | null>(null);
  const [discardTarget, setDiscardTarget] = React.useState<{ id: string; label: string } | null>(
    null,
  );
  const [exportTarget, setExportTarget] = React.useState<{ id: string; name: string } | null>(null);
  const [editorTarget, setEditorTarget] = React.useState<EditorTarget | null>(null);
  const [editorError, setEditorError] = React.useState<string | null>(null);
  const [pendingImport, setPendingImport] = React.useState<PendingImport | null>(null);
  const fileInputRef = React.useRef<HTMLInputElement>(null);

  const handleImportFile = React.useCallback((file: File) => {
    readSpecFile(file, setPendingImport, (message) => notify("template-import", "error", message));
  }, []);

  const handleConfirmImport = React.useCallback(
    (name: string) => {
      if (!pendingImport) return;
      importTemplate.mutate(
        { name, specVersion: pendingImport.specVersion, specJson: pendingImport.specJson },
        {
          onSuccess: () => {
            notify("template-import", "info", `Imported "${name}"`);
            setPendingImport(null);
          },
          onError: (e) => notify("template-import", "error", e.message),
        },
      );
    },
    [pendingImport, importTemplate],
  );

  const handleExport = React.useCallback(
    (id: string, destPath: string) => {
      const target = templates.find((t) => t.id === id);
      exportTemplate.mutate(
        { id, destPath },
        {
          onSuccess: () => {
            notify("template-export", "info", `Exported "${target?.name ?? id}" to ${destPath}`);
            setExportTarget(null);
          },
          onError: (e) => notify("template-export", "error", e.message),
        },
      );
    },
    [templates, exportTemplate],
  );

  const handleSaveSpec = React.useCallback(
    (spec: LaunchSpec) => {
      setEditorError(null);
      const onError = (e: Error) => setEditorError(e.message);
      const specJson = serializeLaunchSpec(spec);
      if (editorTarget?.kind === "create") {
        if (!activeProjectId) return;
        createLaunchTemplate.mutate(
          { projectId: activeProjectId, specVersion: spec.version, specJson },
          { onSuccess: () => setEditorTarget(null), onError },
        );
      } else if (editorTarget?.kind === "edit") {
        applyLaunchSpec.mutate(
          { id: editorTarget.id, specVersion: spec.version, specJson },
          { onSuccess: () => setEditorTarget(null), onError },
        );
      }
    },
    [editorTarget, activeProjectId, createLaunchTemplate, applyLaunchSpec],
  );

  const handleEditLaunchTemplate = React.useCallback(
    (id: string) => {
      const target = launchTemplates.find((t) => t.id === id);
      if (!target) return;
      try {
        setEditorError(null);
        setEditorTarget({ kind: "edit", id, spec: parseLaunchSpec(target.specJson) });
      } catch {
        notify("launch-template", "error", "This template's spec could not be parsed");
      }
    },
    [launchTemplates],
  );

  const registeredHandlers = React.useMemo(
    () => ({
      [ACTION.templatePin]: (args?: CommandArgs) => {
        if (args?.entityId) pinTemplate.mutate({ id: args.entityId });
      },
      [ACTION.templateUnpin]: (args?: CommandArgs) => {
        if (args?.entityId) unpinTemplate.mutate({ id: args.entityId });
      },
      [ACTION.templateExport]: (args?: CommandArgs) => {
        if (!args?.entityId) return;
        const label = typeof args.label === "string" ? args.label : "";
        setExportTarget({ id: args.entityId, name: label });
      },
      [ACTION.templateDelete]: (args?: CommandArgs) => {
        if (!args?.entityId) return;
        const label = typeof args.label === "string" ? args.label : "";
        setDeleteTarget({ id: args.entityId, name: label });
      },
      [ACTION.launchTemplateEdit]: (args?: CommandArgs) => {
        if (args?.entityId) handleEditLaunchTemplate(args.entityId);
      },
      [ACTION.launchTemplateDiscard]: (args?: CommandArgs) => {
        if (!args?.entityId) return;
        const label = typeof args.label === "string" ? args.label : "";
        setDiscardTarget({ id: args.entityId, label });
      },
    }),
    [pinTemplate, unpinTemplate, handleEditLaunchTemplate],
  );
  useRegisterHandlers(registeredHandlers);

  return (
    <div className="flex h-full flex-col">
      <div className="flex h-8 shrink-0 items-center justify-between px-3">
        <span className="text-[0.75rem] font-medium uppercase tracking-[0.05em] text-muted-foreground/70">
          Templates
        </span>
        {isDesktop && (
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
            <Button
              variant="ghost"
              size="xs"
              data-testid="template-import-button"
              onClick={() => fileInputRef.current?.click()}
            >
              <Upload className="size-[var(--icon-sm)]" />
              Import
            </Button>
          </div>
        )}
      </div>

      <ScrollArea className="flex-1 min-h-0">
        <div className="flex flex-col gap-3 py-1">
          <section>
            <SectionHeading>Library</SectionHeading>
            {templates.length === 0 ? (
              <p
                data-testid="templates-empty"
                className="px-3 py-2 text-[0.833rem] text-muted-foreground/50 italic"
              >
                No templates yet
              </p>
            ) : (
              <div className="flex flex-col gap-px" data-testid="templates-library-list">
                {templates.map((t: TemplateView) => (
                  <TemplateRow
                    key={t.id}
                    template={t}
                    isDesktop={isDesktop}
                    onExport={(id) => setExportTarget({ id, name: t.name })}
                    onPin={(id) => pinTemplate.mutate({ id })}
                    onUnpin={(id) => unpinTemplate.mutate({ id })}
                    onRequestDelete={(id, name) => setDeleteTarget({ id, name })}
                  />
                ))}
              </div>
            )}
          </section>

          {activeProjectId && (
            <section>
              <div className="flex items-center justify-between px-3">
                <SectionHeading>This project</SectionHeading>
                {isDesktop && (
                  <Button
                    variant="ghost"
                    size="xs"
                    data-testid="launch-template-create-button"
                    onClick={() => {
                      setEditorError(null);
                      setEditorTarget({ kind: "create" });
                    }}
                  >
                    <Plus className="size-[var(--icon-sm)]" />
                    New
                  </Button>
                )}
              </div>
              {launchTemplates.length === 0 ? (
                <p
                  data-testid="launch-templates-empty"
                  className="px-3 py-2 text-[0.833rem] text-muted-foreground/50 italic"
                >
                  No launch templates for this project
                </p>
              ) : (
                <div className="flex flex-col gap-px" data-testid="launch-templates-list">
                  {launchTemplates.map((t) => (
                    <LaunchTemplateRow
                      key={t.id}
                      id={t.id}
                      label={launchTemplateLabel(t, commandsById)}
                      isDesktop={isDesktop}
                      onEdit={handleEditLaunchTemplate}
                      onRequestDiscard={(id, label) => setDiscardTarget({ id, label })}
                    />
                  ))}
                </div>
              )}
            </section>
          )}
        </div>
      </ScrollArea>

      <SpecEditorDialog
        open={editorTarget !== null}
        title={editorTarget?.kind === "edit" ? "Edit launch template" : "New launch template"}
        spec={editorTarget?.kind === "edit" ? editorTarget.spec : emptySpec()}
        saveError={editorError}
        onOpenChange={(open) => {
          if (!open) {
            setEditorTarget(null);
            setEditorError(null);
          }
        }}
        onSave={handleSaveSpec}
      />

      <ExportTemplateDialog
        target={exportTarget}
        onCancel={() => setExportTarget(null)}
        onExport={handleExport}
      />

      <ImportTemplateDialog
        pending={pendingImport}
        onCancel={() => setPendingImport(null)}
        onConfirm={handleConfirmImport}
      />

      <AlertDialog
        open={deleteTarget != null}
        onOpenChange={(open) => !open && setDeleteTarget(null)}
      >
        <AlertDialogContent data-testid="template-delete-confirm">
          <AlertDialogTitle>Delete {deleteTarget?.name}?</AlertDialogTitle>
          <AlertDialogDescription>
            This will permanently delete the template from your library.
          </AlertDialogDescription>
          <div className="flex gap-2 justify-end">
            <AlertDialogCancel onClick={() => setDeleteTarget(null)}>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => {
                if (!deleteTarget) return;
                discardTemplate.mutate(
                  { id: deleteTarget.id },
                  { onSuccess: () => setDeleteTarget(null) },
                );
              }}
              className="bg-destructive hover:bg-destructive/90"
            >
              Delete
            </AlertDialogAction>
          </div>
        </AlertDialogContent>
      </AlertDialog>

      <AlertDialog
        open={discardTarget != null}
        onOpenChange={(open) => !open && setDiscardTarget(null)}
      >
        <AlertDialogContent data-testid="launch-template-discard-confirm">
          <AlertDialogTitle>Discard {discardTarget?.label}?</AlertDialogTitle>
          <AlertDialogDescription>
            This will remove this launch template from the project.
          </AlertDialogDescription>
          <div className="flex gap-2 justify-end">
            <AlertDialogCancel onClick={() => setDiscardTarget(null)}>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => {
                if (!discardTarget) return;
                discardLaunchTemplate.mutate(
                  { id: discardTarget.id },
                  { onSuccess: () => setDiscardTarget(null) },
                );
              }}
              className="bg-destructive hover:bg-destructive/90"
            >
              Discard
            </AlertDialogAction>
          </div>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}

function SectionHeading({ children }: { children: React.ReactNode }) {
  return (
    <h3 className="px-3 pb-1 text-[0.75rem] font-medium uppercase tracking-[0.05em] text-muted-foreground/70">
      {children}
    </h3>
  );
}
