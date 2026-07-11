import { Tooltip, TooltipContent, TooltipTrigger } from "~/components/ui/tooltip";
import { useSurfaceCommands } from "~/lib/commands/registry";
import { cn } from "~/lib/utils";

// No left buttons: `titleBarStyle: "Overlay"` makes the OS draw the macOS traffic
// lights top-left over a bare drag region. The lights are OS-pinned and unmovable, so
// `--toolbar-height` and the toolbar's `pr-5` are tuned to align with and mirror them.
export function TitleBar() {
  const commands = useSurfaceCommands("titlebar");

  return (
    <div
      data-tauri-drag-region
      style={{ height: "var(--toolbar-height, 2.333rem)" }}
      className="flex shrink-0 items-center border-b border-border/40 bg-background select-none"
    >
      <div data-tauri-drag-region className="flex-1 self-stretch" />
      <div className="flex items-center gap-1 pr-5">
        {commands.map((command) => {
          const Icon = command.icon;
          return (
            <Tooltip key={command.id}>
              <TooltipTrigger
                aria-label={command.title}
                aria-pressed={command.checked ?? undefined}
                onClick={() => command.run()}
                className={cn(
                  "flex h-6 w-6 items-center justify-center rounded-sm text-muted-foreground",
                  "hover:bg-muted hover:text-foreground transition-colors duration-[var(--motion-fast)] ease-standard",
                  "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
                  command.checked && "bg-muted text-foreground",
                )}
              >
                {Icon ? (
                  <Icon className="size-[var(--icon-lg)]" />
                ) : (
                  <span>{command.title.charAt(0)}</span>
                )}
              </TooltipTrigger>
              <TooltipContent>{command.title}</TooltipContent>
            </Tooltip>
          );
        })}
      </div>
    </div>
  );
}
