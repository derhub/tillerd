import { TerminalIcon } from "lucide-react";
import { cn } from "~/lib/utils";

export function EmptyPanel({ onSpawn, disabled }: { onSpawn: () => void; disabled?: boolean }) {
  return (
    <div className="flex flex-col h-full pt-[20%] px-4">
      <p className="text-[0.833rem] text-muted-foreground/50 mb-2 uppercase tracking-wider">
        New surface
      </p>
      <div className="flex flex-col gap-px">
        <button
          type="button"
          disabled={disabled}
          onClick={onSpawn}
          className={cn(
            "flex items-center gap-2 px-2 h-7 rounded-sm text-[0.917rem] text-left",
            "text-muted-foreground hover:text-foreground hover:bg-muted transition-colors duration-[var(--motion-fast)] ease-standard",
            "disabled:opacity-40 disabled:pointer-events-none",
          )}
        >
          <TerminalIcon size={12} />
          New terminal
        </button>
      </div>
    </div>
  );
}
