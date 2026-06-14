import type { NotificationEvent, NotificationSeverity } from "@tillerd/sdk/orchestrator";

/** Display cap for the in-app list; the durable store keeps more (host retention). */
export const MAX_ITEMS = 200;

/** Prepend `event` (most recent first), de-duped by id, bounded to `max`. */
export function boundedPrepend(
  items: NotificationEvent[],
  event: NotificationEvent,
  max: number = MAX_ITEMS,
): NotificationEvent[] {
  const deduped = items.filter((i) => i.id !== event.id);
  return [event, ...deduped].slice(0, max);
}

const CATEGORY_LABEL: Record<string, string> = {
  "surface-started": "Terminal started",
  "surface-stopped": "Terminal stopped",
  "surface-error": "Terminal error",
  "service-up": "Service up",
  "service-down": "Service down",
  "orchestrator-status": "Status",
};

/** A heading for a notification: its title, else a label for its (possibly unknown) category. */
export function notificationHeading(event: NotificationEvent): string {
  if (event.title) return event.title;
  return CATEGORY_LABEL[event.category] ?? "Notification";
}

/** Dot colour per severity, keyed for the chrome. */
export const SEVERITY_DOT: Record<NotificationSeverity, string> = {
  info: "bg-sky-500",
  warning: "bg-amber-500",
  error: "bg-red-500",
};
