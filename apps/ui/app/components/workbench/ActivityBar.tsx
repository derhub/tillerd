import { Tooltip, TooltipContent, TooltipTrigger } from "~/components/ui/tooltip";
import { useSurfaceCommands } from "~/lib/commands/registry";
import { cn } from "~/lib/utils";

// Far-left icon strip. One button per sidebar view, projected from the `activitybar`
// command surface (see defs.ts). A command's `checked` marks the active view; the
// handler (useWorkbenchCommands) switches the view or toggles the sidebar. Always
// visible; every icon is named and tooltipped.
export function ActivityBar() {
  const views = useSurfaceCommands("activitybar");

  return (
    <div
      role="toolbar"
      aria-orientation="vertical"
      aria-label="Views"
      className="flex w-10 shrink-0 flex-col items-center gap-0.5 border-r border-border/40 bg-background py-1"
    >
      {views.map((view) => {
        const Icon = view.icon;
        const active = Boolean(view.checked);
        return (
          <Tooltip key={view.id}>
            <TooltipTrigger
              aria-label={view.title}
              aria-pressed={active}
              onClick={() => view.run()}
              className={cn(
                "relative flex h-9 w-9 items-center justify-center rounded-sm text-muted-foreground",
                "transition-colors duration-[var(--motion-fast)] ease-standard hover:bg-muted hover:text-foreground",
                "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
                active && "text-foreground",
              )}
            >
              {/* Active-view accent edge (DESIGN: primary marks the active indicator). */}
              {active ? (
                <span aria-hidden className="absolute inset-y-1.5 left-0 w-0.5 bg-primary" />
              ) : null}
              {Icon ? (
                <Icon className="size-[var(--icon-lg)]" />
              ) : (
                <span className="text-[0.833rem]">{view.title.charAt(0)}</span>
              )}
            </TooltipTrigger>
            <TooltipContent side="right">{view.title}</TooltipContent>
          </Tooltip>
        );
      })}
    </div>
  );
}
