import { Bell } from "lucide-react";
import { Link } from "react-router";
import type { NotificationEvent } from "@tillerd/sdk/orchestrator";

import { Popover, PopoverContent, PopoverTrigger } from "~/components/ui/popover";
import { useNotifications } from "~/lib/notifications/context";
import { notificationHeading, SEVERITY_DOT } from "~/lib/notifications/store";
import { useDesktopHost } from "~/lib/useDesktopHost";
import { cn } from "~/lib/utils";

function formatTime(ts: number): string {
  return new Date(ts).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

/**
 * Presentational notification list (most recent first). Session rows and actions navigate in-app
 * via `<Link>` — client nav works in the portaled popover. Pure props, so it is unit-tested
 * without the host guard or live store.
 */
export function NotificationPanel({ items }: { items: NotificationEvent[] }) {
  if (items.length === 0) {
    return (
      <div
        data-testid="notification-empty"
        className="p-4 text-center text-xs text-muted-foreground"
      >
        No notifications
      </div>
    );
  }
  return (
    <ul className="flex flex-col divide-y divide-border/40">
      {items.map((n) => (
        <li key={n.id} className="flex flex-col gap-1 px-3 py-2 text-xs">
          <div className="flex items-center gap-2">
            <span className={cn("w-1.5 h-1.5 rounded-full shrink-0", SEVERITY_DOT[n.severity])} />
            <span className="font-medium">{notificationHeading(n)}</span>
            <span className="ml-auto text-muted-foreground">{formatTime(n.ts)}</span>
          </div>
          {n.sessionId ? (
            <Link
              to={`/session/${n.sessionId}`}
              className="text-muted-foreground hover:text-foreground"
            >
              {n.message}
            </Link>
          ) : (
            <span className="text-muted-foreground">{n.message}</span>
          )}
          {n.detail ? <span className="text-muted-foreground/70">{n.detail}</span> : null}
          {n.actions && n.actions.length > 0 ? (
            <div className="flex flex-wrap gap-2 pt-0.5">
              {n.actions.map((a) => (
                <Link
                  key={`${a.label}:${a.to}`}
                  to={a.to}
                  className="text-sky-400 underline hover:text-sky-300"
                >
                  {a.label}
                </Link>
              ))}
            </div>
          ) : null}
        </li>
      ))}
    </ul>
  );
}

/**
 * Bell + unread badge in the shell's bottom-right cluster; clicking opens the notification
 * center popover. Opening clears the unread count. Hidden off the desktop host. Positioning is
 * owned by the cluster (see AppShell).
 */
export function NotificationIndicator() {
  const host = useDesktopHost();
  const { items, unread, markRead } = useNotifications();

  if (host.status === "web") return null;

  const badge = unread > 9 ? "9+" : String(unread);

  return (
    <Popover
      onOpenChange={(open) => {
        if (open) markRead();
      }}
    >
      <PopoverTrigger
        aria-label={`Notifications: ${unread} unread`}
        className="relative flex items-center justify-center rounded-sm bg-black/60 px-2 h-6 select-none"
      >
        <Bell className="size-3.5 text-muted-foreground" />
        {unread > 0 ? (
          <span
            data-testid="notification-unread"
            className="absolute -top-1 -right-1 min-w-3.5 h-3.5 px-0.5 rounded-full bg-red-500 text-[0.6rem] leading-[0.875rem] text-white text-center"
          >
            {badge}
          </span>
        ) : null}
      </PopoverTrigger>
      <PopoverContent className="w-80 max-h-96 overflow-y-auto p-0">
        <NotificationPanel items={items} />
      </PopoverContent>
    </Popover>
  );
}
