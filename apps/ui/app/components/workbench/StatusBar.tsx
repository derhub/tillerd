import { Link } from "@tanstack/react-router";
import { Settings, Undo2 } from "lucide-react";

import { ServiceHealthIndicator } from "~/components/health/ServiceHealthIndicator";
import { NotificationIndicator } from "~/components/notifications/NotificationIndicator";
import { Tooltip, TooltipContent, TooltipTrigger } from "~/components/ui/tooltip";
import { closeSelf } from "~/lib/windows";
import { showBottomPanelTab } from "~/lib/workbench";

import { WorkbenchStatus } from "./WorkbenchStatus";

// The health pill's literal `services: <state>` text is the desktop e2e boot gate --
// keep the format when restyling.
export function StatusBar({ showReattach }: { showReattach: boolean }) {
  return (
    <footer
      aria-label="Status bar"
      className="flex h-[var(--statusbar-height)] shrink-0 items-center gap-2 border-t border-border/40 bg-background px-2"
    >
      <ServiceHealthIndicator />
      <WorkbenchStatus />
      <div className="ml-auto flex items-center gap-2">
        {showReattach ? (
          <button
            type="button"
            onClick={() => void closeSelf()}
            aria-label="Re-attach"
            className="flex items-center gap-1 px-2 h-5 text-[0.833rem] rounded-sm text-muted-foreground hover:text-foreground hover:bg-muted transition-colors duration-[var(--motion-fast)] ease-standard focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
          >
            <Undo2 className="size-[var(--icon-sm)]" />
            <span>Re-attach</span>
          </button>
        ) : null}
        <NotificationIndicator onActivate={() => showBottomPanelTab("notifications")} />
        {/* Retired popover -> the settings editor is a route now (ui-settings-editor spec). */}
        <Tooltip>
          <TooltipTrigger
            render={
              <Link
                to="/settings"
                aria-label="Settings"
                className="flex items-center justify-center rounded-sm w-6 h-6 text-muted-foreground hover:text-foreground hover:bg-muted transition-colors duration-[var(--motion-fast)] ease-standard focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
              />
            }
          >
            <Settings className="size-[var(--icon-md)]" />
          </TooltipTrigger>
          <TooltipContent>Settings</TooltipContent>
        </Tooltip>
      </div>
    </footer>
  );
}
