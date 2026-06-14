import { useEffect } from "react";
import { Undo2 } from "lucide-react";
import { SettingsProvider } from "~/lib/settings/context";
import { DesktopTerminalPane } from "~/components/DesktopTerminalPane";
import { armReattachOnClose, closeSelf, emitReattachPanel } from "~/lib/windows";

// The child window for a detached panel: one surface plus a Re-attach action. The pane uses the
// same `(session, placement)` identity, so the host's revisit path re-binds the live PTY and
// replays scrollback — no surface is cloned or restarted.
export function DetachedWindow({ sessionId, placement }: { sessionId: string; placement: string }) {
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void armReattachOnClose(() => emitReattachPanel({ sessionId, placement })).then(
      (u) => (unlisten = u),
    );
    return () => unlisten?.();
  }, [sessionId, placement]);

  return (
    <SettingsProvider>
      <div className="h-dvh w-full flex flex-col overflow-hidden">
        <div
          className="flex items-center shrink-0 px-3 gap-1.5"
          style={{ height: "var(--panel-header-height, 2.5rem)" }}
        >
          <span className="truncate text-muted-foreground/60 flex-1 select-none text-[0.833rem] font-medium tracking-wider uppercase">
            Terminal
          </span>
          <button
            type="button"
            onClick={() => void closeSelf()}
            aria-label="Re-attach"
            className="flex items-center gap-1 px-2 h-6 text-[0.833rem] rounded-sm text-muted-foreground hover:text-foreground hover:bg-muted transition-colors duration-[var(--motion-fast)] ease-standard"
          >
            <Undo2 size={12} />
            <span>Re-attach</span>
          </button>
        </div>
        <div className="flex-1 min-h-0 overflow-hidden">
          <DesktopTerminalPane
            sessionId={sessionId}
            placement={placement}
            cwd=""
            detachOnUnmount={false}
          />
        </div>
      </div>
    </SettingsProvider>
  );
}
