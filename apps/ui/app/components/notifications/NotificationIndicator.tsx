import type { NotificationWire } from "@tillerd/client-bindings";
import type { ReactNode } from "react";

import { useMutation, useQuery } from "@tanstack/react-query";
import { Link } from "@tanstack/react-router";
import { command, query } from "@tillerd/client-bindings";
import { Bell, CheckCheck, CircleCheck, Clock, Trash2 } from "lucide-react";

import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "~/components/ui/dropdown-menu";
import { Tooltip, TooltipContent, TooltipTrigger } from "~/components/ui/tooltip";
import {
  clearNotifications,
  removeNotification,
  useNotifications,
} from "~/lib/notifications/context";
import { notificationHeading, SEVERITY_DOT, SNOOZE_OPTIONS } from "~/lib/notifications/store";
import { useDesktopHost } from "~/lib/useDesktopHost";
import { cn } from "~/lib/utils";

function formatTime(ts: number): string {
  return new Date(ts).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

// Small icon-only action button shared by the header and row controls. Tooltip carries the
// accessible name's human-readable echo; aria-label is the actual accessible name so it works
// with no pointer (tooltips are hover/focus-only decoration, not a substitute for ARIA).
function ActionButton({
  label,
  disabled,
  onClick,
  children,
}: {
  label: string;
  disabled?: boolean;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <Tooltip>
      <TooltipTrigger
        type="button"
        aria-label={label}
        disabled={disabled}
        onClick={onClick}
        className="rounded-sm p-1 text-muted-foreground hover:text-foreground hover:bg-muted transition-colors duration-[var(--motion-fast)] ease-standard focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-40"
      >
        {children}
      </TooltipTrigger>
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  );
}

export function NotificationPanel({ items }: { items: NotificationWire[] }) {
  // Server truth for the unread count (spec: a snoozed notification stops counting as unread
  // until it elapses) -- this reads notification_count_unread directly rather than re-deriving
  // "unread" from `items`, which carries no read/snooze state on the wire.
  const unreadCount = useQuery(query("notificationCountUnread"));

  const markReadMutation = useMutation(command("notificationMarkRead"));
  const markAllReadMutation = useMutation(command("notificationMarkAllRead"));
  const disregardMutation = useMutation(command("notificationDisregard"));
  const disregardAllMutation = useMutation(command("notificationDisregardAll"));
  const snoozeMutation = useMutation(command("notificationSnooze"));

  const handleDisregard = (id: string) => {
    disregardMutation.mutate({ id }, { onSuccess: () => removeNotification(id) });
  };
  const handleDisregardAll = () => {
    disregardAllMutation.mutate(undefined, { onSuccess: () => clearNotifications() });
  };
  const handleSnooze = (id: string, minutes: number) => {
    snoozeMutation.mutate({ id, snoozeUntil: Date.now() + minutes * 60_000 });
  };

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
    <div className="flex flex-col">
      <div className="flex items-center justify-between gap-2 border-b border-border/40 px-3 py-1.5 text-xs">
        <span className="text-muted-foreground">
          {typeof unreadCount.data === "number" && unreadCount.data > 0
            ? `${unreadCount.data} unread`
            : null}
        </span>
        <div className="flex items-center gap-1">
          <ActionButton label="Mark all read" onClick={() => markAllReadMutation.mutate(undefined)}>
            <CheckCheck className="size-[var(--icon-md)]" />
          </ActionButton>
          <ActionButton label="Disregard all" onClick={handleDisregardAll}>
            <Trash2 className="size-[var(--icon-md)]" />
          </ActionButton>
        </div>
      </div>
      <ul className="flex flex-col divide-y divide-border/40">
        {items.map((n) => (
          <li key={n.id} className="flex flex-col gap-1 px-3 py-2 text-xs">
            <div className="flex items-center gap-2">
              <span
                className={cn(
                  "w-1.5 h-1.5 rounded-full shrink-0",
                  SEVERITY_DOT[n.severity as "info" | "warning" | "error"],
                )}
              />
              <span className="font-medium">{notificationHeading(n)}</span>
              <span className="ml-auto text-muted-foreground">{formatTime(n.ts)}</span>
            </div>
            {n.sessionId ? (
              <Link
                to={`/session/${n.sessionId}` as never}
                className="text-muted-foreground hover:text-foreground"
              >
                {n.message}
              </Link>
            ) : (
              <span className="text-muted-foreground">{n.message}</span>
            )}
            {n.detail ? <span className="text-muted-foreground/70">{n.detail}</span> : null}
            <div className="flex items-center gap-0.5 pt-0.5">
              <ActionButton
                label={`Mark read: ${notificationHeading(n)}`}
                onClick={() => markReadMutation.mutate({ id: n.id })}
              >
                <CircleCheck className="size-[var(--icon-md)]" />
              </ActionButton>
              <ActionButton
                label={`Disregard: ${notificationHeading(n)}`}
                onClick={() => handleDisregard(n.id)}
              >
                <Trash2 className="size-[var(--icon-md)]" />
              </ActionButton>
              <DropdownMenu>
                <DropdownMenuTrigger
                  aria-label={`Snooze: ${notificationHeading(n)}`}
                  className="rounded-sm p-1 text-muted-foreground hover:text-foreground hover:bg-muted transition-colors duration-[var(--motion-fast)] ease-standard focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                >
                  <Clock className="size-[var(--icon-md)]" />
                </DropdownMenuTrigger>
                <DropdownMenuContent align="start">
                  {SNOOZE_OPTIONS.map((opt) => (
                    <DropdownMenuItem key={opt.minutes} onClick={() => handleSnooze(n.id, opt.minutes)}>
                      {opt.label}
                    </DropdownMenuItem>
                  ))}
                </DropdownMenuContent>
              </DropdownMenu>
            </div>
          </li>
        ))}
      </ul>
    </div>
  );
}

// Status-bar bell. The full feed now lives in the bottom panel's Notifications tab,
// so clicking opens that tab (via `onActivate`) instead of a popover; opening also
// clears the unread count. The bell keeps the unread badge and its accessible name.
export function NotificationIndicator({ onActivate }: { onActivate?: () => void }) {
  const host = useDesktopHost();
  const { unread, markRead } = useNotifications();

  if (host.status === "web") return null;

  const badge = unread > 9 ? "9+" : String(unread);
  const label = `Notifications: ${unread} unread`;

  return (
    <Tooltip>
      <TooltipTrigger
        type="button"
        aria-label={label}
        onClick={() => {
          markRead();
          onActivate?.();
        }}
        className="relative flex items-center justify-center rounded-sm px-2 h-6 select-none text-muted-foreground hover:text-foreground hover:bg-muted transition-colors duration-[var(--motion-fast)] ease-standard focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
      >
        <Bell className="size-[var(--icon-md)]" />
        {unread > 0 ? (
          <span
            data-testid="notification-unread"
            className="absolute -top-1 -right-1 min-w-3.5 h-3.5 px-0.5 rounded-full bg-red-500 text-[0.6rem] leading-[0.875rem] text-white text-center"
          >
            {badge}
          </span>
        ) : null}
      </TooltipTrigger>
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  );
}
