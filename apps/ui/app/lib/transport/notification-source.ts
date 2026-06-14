import type { NotificationEvent } from "@tillerd/sdk/orchestrator";
import { NOTIFICATION_EVENT } from "@tillerd/sdk/orchestrator";

import { isDesktopHost, loadTauriCore } from "./core";
import type { TauriCore } from "./tauri";

/** Tauri command returning durable notification history, most recent first. */
export const NOTIFICATIONS_LIST = "notifications_list";

/**
 * Host-agnostic source the notification center reads through. The desktop adapter is
 * {@link TauriNotificationSource}; a server/web adapter satisfies the same contract
 * (`history` -> index endpoint, `subscribe` -> SSE/WS) without changing the feed.
 */
export interface NotificationSource {
  history(): Promise<NotificationEvent[]>;
  subscribe(handler: (event: NotificationEvent) => void): Promise<() => void>;
}

/** Desktop (Tauri) {@link NotificationSource}: the `notifications_list` command + the event. */
export class TauriNotificationSource implements NotificationSource {
  constructor(private readonly core: TauriCore) {}

  history(): Promise<NotificationEvent[]> {
    return this.core.invoke<NotificationEvent[]>(NOTIFICATIONS_LIST);
  }

  async subscribe(handler: (event: NotificationEvent) => void): Promise<() => void> {
    const { listen } = await import("@tauri-apps/api/event");
    return listen<NotificationEvent>(NOTIFICATION_EVENT, (e) => handler(e.payload));
  }
}

/**
 * Resolve the notification source for the current host. Returns `null` off the desktop
 * host: the server/web adapter is deferred, and the bell hides until it lands.
 */
export async function loadNotificationSource(): Promise<NotificationSource | null> {
  if (!isDesktopHost()) return null;
  return new TauriNotificationSource(await loadTauriCore());
}
