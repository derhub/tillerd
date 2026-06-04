import { NavLink, useNavigate } from "react-router";
import { Plus } from "lucide-react";
import { ScrollArea } from "~/components/ui/scroll-area";
import { cn } from "~/lib/utils";
import { useSpawnSession } from "~/lib/useSpawnSession";
import { useDesktopHost } from "~/lib/useDesktopHost";

type Session = { id: string; cwd?: string };

export function SessionSidebar({ sessions }: { sessions: Session[] }) {
  const host = useDesktopHost();
  const navigate = useNavigate();
  const web = useSpawnSession();
  // On desktop a new session is started by the terminal pane at /session/new; on web the existing
  // WebSocket spawn flow drives it.
  const desktopReady = host.status === "ready";
  const spawn = desktopReady ? () => void navigate("/session/new") : web.spawn;
  const spawning = desktopReady ? false : web.spawning;

  return (
    <div className="flex flex-col h-full">
      {/* New session — left-aligned ghost button */}
      <div className="px-3 pt-2 pb-1 shrink-0">
        <button
          type="button"
          onClick={spawn}
          disabled={spawning}
          className={cn(
            "flex items-center gap-1.5 px-2 h-6 text-[0.917rem] rounded-sm transition-colors",
            "text-muted-foreground hover:text-foreground hover:bg-muted",
            "disabled:opacity-40 disabled:cursor-not-allowed",
          )}
        >
          <Plus size={11} strokeWidth={2} />
          {spawning ? "Connecting…" : "New session"}
        </button>
      </div>

      <ScrollArea className="flex-1 min-h-0">
        {sessions.length === 0 ? (
          <p className="px-3 py-3 text-[0.917rem] text-muted-foreground/50 italic">
            No active sessions
          </p>
        ) : (
          <div className="flex flex-col gap-px py-1">
            {sessions.map((s) => (
              <NavLink
                key={s.id}
                to={`/session/${s.id}`}
                className={({ isActive }) =>
                  cn(
                    "flex items-center gap-2 px-3 h-7 text-[0.917rem] rounded-sm transition-colors",
                    isActive
                      ? "bg-muted text-foreground"
                      : "text-muted-foreground hover:text-foreground hover:bg-muted/50",
                  )
                }
              >
                {/* Active indicator dot */}
                <span className="w-1.5 h-1.5 rounded-full shrink-0 bg-emerald-500/80" />
                <span className="font-mono text-[0.833rem] shrink-0">{s.id.slice(0, 8)}</span>
                {s.cwd && (
                  <span className="truncate text-muted-foreground/60 text-[0.833rem]">
                    {sessionBasename(s.cwd)}
                  </span>
                )}
              </NavLink>
            ))}
          </div>
        )}
      </ScrollArea>
    </div>
  );
}

function sessionBasename(cwd: string): string {
  const parts = cwd.replace(/\\/g, "/").split("/");
  return parts[parts.length - 1] || cwd;
}
