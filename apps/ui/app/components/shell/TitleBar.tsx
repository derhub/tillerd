import { useSurfaceCommands } from "~/lib/commands/registry";
import { cn } from "~/lib/utils";

// The top band of the window. The OS draws the native window controls at the top
// left (macOS traffic lights, via `titleBarStyle: "Overlay"`), so this component
// draws no control buttons -- the left area is just a drag region the controls sit
// over. The command toolbar is right-aligned at the end of the title bar. The bar
// height matches the native macOS title bar (h-7 / 28px) so the toolbar centers on
// the same line as the traffic lights, and its right inset mirrors their left
// inset. The toolbar projects the commands tagged for the `titlebar` surface, so
// adding a button is a command-definition edit, not a change here.
export function TitleBar() {
  const commands = useSurfaceCommands("titlebar");

  return (
    <div
      data-tauri-drag-region
      className="flex h-10 shrink-0 items-start border-b border-border/40 bg-background select-none"
    >
      <div data-tauri-drag-region className="flex-1 self-stretch" />
      {/* Top strip (h-7) keeps the toolbar's center on the traffic-lights line while
          the bar itself has proper height below. */}
      <div className="flex h-7 items-center gap-1 pr-3">
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
              {Icon ? <Icon size={15} /> : <span>{command.title.charAt(0)}</span>}
            </button>
          );
        })}
      </div>
    </div>
  );
}
