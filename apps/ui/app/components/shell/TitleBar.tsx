import { useSurfaceCommands } from "~/lib/commands/registry";
import { isMac } from "~/lib/platform";
import { isDesktopHost } from "~/lib/transport/core";
import { cn } from "~/lib/utils";

// The top band of the window. The OS draws the native window controls here
// (macOS keeps its traffic lights via `titleBarStyle: "Overlay"`), so this
// component adds no control buttons -- it reserves space for the native controls
// on the left, is a drag region, and renders the command toolbar inline beside
// them. The toolbar projects the commands tagged for the `titlebar` surface, so
// adding a button is a command-definition edit, not a change here.
export function TitleBar() {
  const commands = useSurfaceCommands("titlebar");
  // macOS traffic lights overlay the top-left; reserve room so the toolbar sits
  // to their right rather than under them.
  const reserveTrafficLights = isMac && isDesktopHost();

  return (
    <div
      data-tauri-drag-region
      className={cn(
        "flex h-9 shrink-0 items-center border-b border-border/40 bg-background select-none",
        reserveTrafficLights ? "pl-20" : "pl-2",
      )}
    >
      <div className="flex items-center gap-0.5">
        {commands.map((command) => {
          const Icon = command.icon;
          return (
            <button
              key={command.id}
              type="button"
              aria-label={command.title}
              aria-pressed={command.checked ?? undefined}
              title={command.title}
              onClick={command.run}
              className={cn(
                "flex h-6 w-6 items-center justify-center rounded-sm text-muted-foreground",
                "hover:bg-muted hover:text-foreground transition-colors duration-[var(--motion-fast)] ease-standard",
                command.checked && "bg-muted text-foreground",
              )}
            >
              {Icon ? <Icon size={14} /> : <span>{command.title.charAt(0)}</span>}
            </button>
          );
        })}
      </div>
      <div data-tauri-drag-region className="flex-1 self-stretch" />
    </div>
  );
}
