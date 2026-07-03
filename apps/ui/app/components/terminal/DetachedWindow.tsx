import { Undo2 } from "lucide-react";
import React from "react";

import { DesktopTerminalPane } from "~/components/terminal/DesktopTerminalPane";
import { SettingsProvider } from "~/lib/settings/context";
import { subscribe } from "~/lib/subscribe";
import { armReattachOnClose, closeSelf, emitReattachPanel } from "~/lib/windows";

// Same (sessionId, placement) identity as the host panel: the revisit path re-binds the live PTY
// and replays scrollback without cloning or restarting the surface.
export function DetachedWindow({ sessionId, placement }: { sessionId: string; placement: string }) {
  React.useEffect(() => {
    return subscribe(armReattachOnClose(() => emitReattachPanel({ sessionId, placement })));
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
