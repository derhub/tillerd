import type { CommandView, SpawnCommandRef } from "@tillerd/client-bindings";

import { TerminalIcon } from "lucide-react";

import { cn } from "~/lib/utils";

// Empty panel picker (ui-panel-compound "Empty panel picker", usability pass 12.4): lists the
// available surface kinds (terminal login shell, the only kind in 0.x) plus the command library
// (pinned first) so picking a library entry spawns a terminal running that command instead of a
// bare shell. Sorts defensively rather than trusting caller order -- "pinned first" is this
// picker's own contract, not something it should silently inherit.
function sortPinnedFirst(commands: readonly CommandView[]): CommandView[] {
  return [...commands].sort((a, b) => Number(b.pinned) - Number(a.pinned));
}

// One picker card. Both the login-shell kind and every library command render through this so a
// styling/a11y change lands on every row at once (the two branches previously duplicated the
// className/markup).
function PickerItem({
  title,
  subtitle,
  testid,
  disabled,
  onClick,
}: {
  title: string;
  subtitle: string;
  testid: string;
  disabled?: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      data-testid={testid}
      className={cn(
        "flex items-center gap-3 rounded-sm border border-border/60 px-3 py-2.5 text-left",
        "hover:border-primary/50 hover:bg-muted transition-colors duration-[var(--motion-fast)] ease-standard",
        "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
        "disabled:opacity-40 disabled:pointer-events-none",
      )}
    >
      <TerminalIcon className="size-[var(--icon-lg)] text-muted-foreground shrink-0" />
      <span className="flex flex-col min-w-0">
        <span className="text-[0.917rem] text-foreground truncate">{title}</span>
        <span className="text-[0.75rem] text-muted-foreground/60 truncate">{subtitle}</span>
      </span>
    </button>
  );
}

export function EmptyPanel({
  onSpawn,
  disabled,
  commands = [],
}: {
  onSpawn: (command?: SpawnCommandRef) => void;
  disabled?: boolean;
  commands?: readonly CommandView[];
}) {
  return (
    <div
      className="flex flex-col h-full items-center justify-center gap-4 px-4"
      data-testid="empty-panel-picker"
    >
      <p className="text-[0.833rem] text-muted-foreground/50 uppercase tracking-wider">
        New surface
      </p>
      <div className="flex flex-col gap-1.5 w-full max-w-60 max-h-[60vh] overflow-y-auto">
        <PickerItem
          title="New terminal"
          subtitle="Interactive shell surface"
          testid="empty-panel-kind-terminal"
          disabled={disabled}
          onClick={() => onSpawn()}
        />
        {sortPinnedFirst(commands).map((cmd) => (
          <PickerItem
            key={cmd.id}
            title={cmd.name}
            subtitle={cmd.cli}
            testid={`empty-panel-command-${cmd.id}`}
            disabled={disabled}
            onClick={() => onSpawn({ libraryRef: cmd.id })}
          />
        ))}
      </div>
    </div>
  );
}
