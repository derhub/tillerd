/**
 * A user-relevant lifecycle category surfaced in the notification center. The
 * known values drive default titles/severity; the open `string` arm keeps the
 * taxonomy extensible (future event kinds -- agent, diff, workflow) without a
 * schema break.
 */
export type NotificationCategory =
  | "surface-started"
  | "surface-stopped"
  | "surface-error"
  | "service-up"
  | "service-down"
  | "orchestrator-status"
  // eslint-disable-next-line @typescript-eslint/ban-types -- open union keeps literal autocomplete while allowing future categories
  | (string & {});

export type NotificationSeverity = "info" | "warning" | "error";

/** An extra activatable action beyond the default session click-through. */
export interface NotificationAction {
  label: string;
  /** In-app route to navigate to when activated (e.g. `/session/abc`, `/logs?service=x`). */
  to: string;
}

/**
 * One user-facing notification. The host derives these from existing lifecycle
 * signals and pushes them over {@link NOTIFICATION_EVENT}; the in-app center keeps
 * a bounded history of them. `sessionId` enables the default click-through; the
 * optional `title`/`detail`/`actions` carry richer content as it arrives.
 */
export interface NotificationEvent {
  id: string;
  category: NotificationCategory;
  severity: NotificationSeverity;
  /** Optional heading; the center falls back to a category label when absent. */
  title?: string;
  /** One-line summary, always present (unlike `title`/`detail`). */
  message: string;
  detail?: string;
  /** Epoch milliseconds when the event occurred. */
  ts: number;
  /** Present when the event concerns a surface -- drives the default click-through. */
  sessionId?: string;
  surfaceId?: string;
  actions?: NotificationAction[];
}

/** Host-pushed event carrying a single {@link NotificationEvent}. */
export const NOTIFICATION_EVENT = "notification://event";
