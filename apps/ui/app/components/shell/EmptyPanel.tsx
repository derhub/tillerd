import { TerminalIcon } from "lucide-react";

import { cn } from "~/lib/utils";

// Surface kinds a picker offers. Terminal is the only kind in 0.x (roadmap: diff surfaces);
// keyed by the value the backend spawn command produces, not a display label.
const SURFACE_KINDS = [
  {
    id: "terminal",
    label: "New terminal",
    description: "Interactive shell surface",
    icon: TerminalIcon,
  },
] as const;

// Empty panel picker (ui-panel-compound "Empty panel picker"): lists the available surface kinds
// and spawns the chosen one into this leaf's placement. Only one kind exists in 0.x, but the list
// shape carries forward to later surface kinds without a rewrite.
export function EmptyPanel({ onSpawn, disabled }: { onSpawn: () => void; disabled?: boolean }) {
  return (
    <div
      className="flex flex-col h-full items-center justify-center gap-4 px-4"
      data-testid="empty-panel-picker"
    >
      <p className="text-[0.833rem] text-muted-foreground/50 uppercase tracking-wider">
        New surface
      </p>
      <div className="flex flex-col gap-1.5 w-full max-w-60">
        {SURFACE_KINDS.map((kind) => (
          <button
            key={kind.id}
            type="button"
            disabled={disabled}
            onClick={onSpawn}
            data-testid={`empty-panel-kind-${kind.id}`}
            className={cn(
              "flex items-center gap-3 rounded-sm border border-border/60 px-3 py-2.5 text-left",
              "hover:border-primary/50 hover:bg-muted transition-colors duration-[var(--motion-fast)] ease-standard",
              "disabled:opacity-40 disabled:pointer-events-none",
            )}
          >
            <kind.icon size={16} className="text-muted-foreground shrink-0" />
            <span className="flex flex-col min-w-0">
              <span className="text-[0.917rem] text-foreground">{kind.label}</span>
              <span className="text-[0.75rem] text-muted-foreground/60 truncate">
                {kind.description}
              </span>
            </span>
          </button>
        ))}
      </div>
    </div>
  );
}
