import type { TemplateView } from "@tillerd/client-bindings";

import { Download, Pin, PinOff, Trash2 } from "lucide-react";

import { EntityContextMenu } from "~/components/shell/EntityContextMenu";
import { Badge } from "~/components/ui/badge";
import { Tooltip, TooltipContent, TooltipTrigger } from "~/components/ui/tooltip";
import { cn } from "~/lib/utils";

const ROW_ACTION_CLASS =
  "opacity-0 group-hover:opacity-100 flex items-center justify-center w-6 h-6 rounded-sm transition-all duration-[var(--motion-fast)] ease-standard focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring";

// One portable-library template row. Prebuilt rows never offer Delete (`canEdit`
// gates the def's `when` and this row's own hover button identically); there is
// no update op for library templates, so there is no Edit affordance here.
export function TemplateRow({
  template,
  isDesktop,
  onExport,
  onPin,
  onUnpin,
  onRequestDelete,
}: {
  template: TemplateView;
  isDesktop: boolean;
  onExport: (id: string) => void;
  onPin: (id: string) => void;
  onUnpin: (id: string) => void;
  onRequestDelete: (id: string, name: string) => void;
}) {
  const canDelete = template.origin === "custom";

  return (
    <EntityContextMenu
      entityId={template.id}
      entityKind="template"
      args={{ label: template.name }}
      guards={{ "menu.canEdit": canDelete, "menu.pinned": template.pinned }}
      disabled={!isDesktop}
      className="group flex items-center gap-2 h-8 px-3 rounded-sm"
      data-testid="template-row"
      data-template-id={template.id}
      data-template-origin={template.origin}
    >
      <span
        className="flex-1 min-w-0 truncate text-[0.833rem] text-foreground"
        data-testid="template-name"
      >
        {template.name}
      </span>

      {template.pinned && (
        <Pin
          strokeWidth={2}
          aria-hidden
          data-testid="template-pinned-indicator"
          className="shrink-0 size-[var(--icon-sm)] text-muted-foreground/40"
        />
      )}

      <Badge variant="outline" className="shrink-0" data-testid="template-origin-badge">
        {template.origin}
      </Badge>

      {isDesktop && (
        <div className="flex items-center gap-0.5 shrink-0">
          <Tooltip>
            <TooltipTrigger
              aria-label={`Export ${template.name}`}
              onClick={(e) => {
                e.stopPropagation();
                onExport(template.id);
              }}
              className={cn(ROW_ACTION_CLASS, "text-muted-foreground/50 hover:text-foreground hover:bg-muted")}
            >
              <Download className="size-[var(--icon-sm)]" strokeWidth={2} />
            </TooltipTrigger>
            <TooltipContent>Export</TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger
              aria-label={template.pinned ? `Unpin ${template.name}` : `Pin ${template.name}`}
              onClick={(e) => {
                e.stopPropagation();
                if (template.pinned) onUnpin(template.id);
                else onPin(template.id);
              }}
              className={cn(ROW_ACTION_CLASS, "text-muted-foreground/50 hover:text-foreground hover:bg-muted")}
            >
              {template.pinned ? (
                <PinOff className="size-[var(--icon-sm)]" strokeWidth={2} />
              ) : (
                <Pin className="size-[var(--icon-sm)]" strokeWidth={2} />
              )}
            </TooltipTrigger>
            <TooltipContent>{template.pinned ? "Unpin" : "Pin"}</TooltipContent>
          </Tooltip>
          {canDelete && (
            <Tooltip>
              <TooltipTrigger
                aria-label={`Delete ${template.name}`}
                onClick={(e) => {
                  e.stopPropagation();
                  onRequestDelete(template.id, template.name);
                }}
                className={cn(ROW_ACTION_CLASS, "text-muted-foreground/50 hover:text-destructive hover:bg-destructive/10")}
              >
                <Trash2 className="size-[var(--icon-sm)]" strokeWidth={2} />
              </TooltipTrigger>
              <TooltipContent>Delete</TooltipContent>
            </Tooltip>
          )}
        </div>
      )}
    </EntityContextMenu>
  );
}
