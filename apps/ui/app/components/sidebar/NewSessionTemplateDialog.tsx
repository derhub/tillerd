import { useQuery } from "@tanstack/react-query";
import React from "react";

import type { TemplateSelection } from "~/lib/newSessionTemplate";

import { Dialog, DialogContent, DialogDescription, DialogTitle } from "~/components/ui/dialog";
import { commandListQuery } from "~/lib/data/commands";
import { launchTemplateListQuery, templateListQuery } from "~/lib/data/templates";
import { describeLaunchSpec } from "~/lib/launchSpec";
import { cn } from "~/lib/utils";

export interface NewSessionTemplateTarget {
  projectId: string;
  projectName: string;
}

const OPTION_CLASS = cn(
  "text-left text-[0.833rem] px-2 h-7 rounded-sm truncate transition-colors duration-[var(--motion-fast)] ease-standard",
  "text-muted-foreground hover:text-foreground hover:bg-muted",
);

// New-session template picker (ui-template-manager / ui-settings-editor specs):
// offers an empty session, the project's own launch templates, and the portable
// library, in that order. The plain "+" new-session control bypasses this dialog
// entirely when a default template is configured (see SessionSidebar) -- it only
// opens from the explicit "New session from template..." row action.
export function NewSessionTemplateDialog({
  target,
  onCancel,
  onSelect,
}: {
  target: NewSessionTemplateTarget | null;
  onCancel: () => void;
  onSelect: (selection: TemplateSelection) => void;
}) {
  const open = Boolean(target);
  const { data: launchTemplates = [] } = useQuery({
    ...launchTemplateListQuery(target?.projectId ?? ""),
    enabled: open,
  });
  const { data: libraryTemplates = [] } = useQuery({ ...templateListQuery(), enabled: open });
  const { data: commands = [] } = useQuery({ ...commandListQuery(), enabled: open });
  const commandsById = React.useMemo(() => new Map(commands.map((c) => [c.id, c])), [commands]);

  if (!target) return null;

  return (
    <Dialog open onOpenChange={(open) => !open && onCancel()}>
      <DialogContent data-testid="new-session-template-picker">
        <DialogTitle>New session in {target.projectName}</DialogTitle>
        <DialogDescription>Choose what to start the session from.</DialogDescription>
        <div className="flex flex-col gap-3 max-h-80 overflow-y-auto">
          <section className="flex flex-col gap-px">
            <button
              type="button"
              data-testid="new-session-option-empty"
              onClick={() => onSelect({ kind: "empty" })}
              className={OPTION_CLASS}
            >
              Empty session
            </button>
          </section>

          <section>
            <SectionHeading>This project</SectionHeading>
            {launchTemplates.length === 0 ? (
              <p className="px-2 py-2 text-[0.833rem] text-muted-foreground/50 italic">
                No launch templates for this project
              </p>
            ) : (
              <div className="flex flex-col gap-px">
                {launchTemplates.map((t) => (
                  <button
                    key={t.id}
                    type="button"
                    data-testid="new-session-option-launch"
                    data-template-id={t.id}
                    onClick={() => onSelect({ kind: "launch", id: t.id })}
                    className={OPTION_CLASS}
                  >
                    {describeLaunchSpec(t.specJson, (id) => commandsById.get(id)?.name)}
                  </button>
                ))}
              </div>
            )}
          </section>

          <section>
            <SectionHeading>Library</SectionHeading>
            {libraryTemplates.length === 0 ? (
              <p className="px-2 py-2 text-[0.833rem] text-muted-foreground/50 italic">
                No templates yet
              </p>
            ) : (
              <div className="flex flex-col gap-px">
                {libraryTemplates.map((t) => (
                  <button
                    key={t.id}
                    type="button"
                    data-testid="new-session-option-library"
                    data-template-id={t.id}
                    onClick={() => onSelect({ kind: "library", id: t.id })}
                    className={OPTION_CLASS}
                  >
                    {t.name}
                  </button>
                ))}
              </div>
            )}
          </section>
        </div>
      </DialogContent>
    </Dialog>
  );
}

function SectionHeading({ children }: { children: React.ReactNode }) {
  return (
    <h3 className="px-2 pb-1 text-[0.75rem] font-medium uppercase tracking-[0.05em] text-muted-foreground/70">
      {children}
    </h3>
  );
}
