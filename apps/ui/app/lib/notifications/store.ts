import type { NotificationWire } from "@tillerd/client-bindings";

export const MAX_ITEMS = 200;

export function boundedPrepend(
  items: NotificationWire[],
  event: NotificationWire,
  max: number = MAX_ITEMS,
): NotificationWire[] {
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

export function notificationHeading(event: NotificationWire): string {
  if (event.title) return event.title;
  return CATEGORY_LABEL[event.category] ?? "Notification";
}

export const SEVERITY_DOT: Record<"info" | "warning" | "error", string> = {
  info: "bg-sky-500",
  warning: "bg-amber-500",
  error: "bg-red-500",
};

// Snooze durations the per-row picker offers (spec: "a chosen duration").
export interface SnoozeOption {
  readonly label: string;
  readonly minutes: number;
}

export const SNOOZE_OPTIONS: readonly SnoozeOption[] = [
  { label: "15m", minutes: 15 },
  { label: "1h", minutes: 60 },
  { label: "8h", minutes: 8 * 60 },
];
