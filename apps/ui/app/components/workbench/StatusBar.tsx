import { Undo2 } from "lucide-react";

import { ServiceHealthIndicator } from "~/components/health/ServiceHealthIndicator";
import { NotificationIndicator } from "~/components/notifications/NotificationIndicator";
import { SettingsPanel } from "~/components/settings/SettingsPanel";
import { closeSelf } from "~/lib/windows";
import { showBottomPanelTab } from "~/lib/workbench";

import { WorkbenchStatus } from "./WorkbenchStatus";

// The health pill's literal `services: <state>` text is the desktop e2e boot gate --
// keep the format when restyling.
export function StatusBar({ showReattach }: { showReattach: boolean }) {
  return (
    <footer
      aria-label="Status bar"
      className="flex h-7 shrink-0 items-center gap-2 border-t border-border/40 bg-background px-2"
    >
      <ServiceHealthIndicator />
      <WorkbenchStatus />
      <div className="ml-auto flex items-center gap-2">
        {showReattach ? (
          <button
            type="button"
            onClick={() => void closeSelf()}
            aria-label="Re-attach"
            className="flex items-center gap-1 px-2 h-5 text-[0.833rem] rounded-sm text-muted-foreground hover:text-foreground hover:bg-muted transition-colors duration-[var(--motion-fast)] ease-standard"
          >
            <Undo2 size={12} />
            <span>Re-attach</span>
          </button>
        ) : null}
        <NotificationIndicator onActivate={() => showBottomPanelTab("notifications")} />
        <SettingsPanel />
      </div>
    </footer>
  );
}
