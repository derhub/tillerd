import type { NotificationWire } from "@tillerd/client-bindings";

import { commands, ensureResult, events } from "@tillerd/client-bindings";

import { isDesktopHost } from "./core";

export interface NotificationSource {
  history(): Promise<NotificationWire[]>;
  subscribe(handler: (event: NotificationWire) => void): Promise<() => void>;
}

// Desktop notification feed over the generated client: list via `commands`, live via `events`.
// Resolves null off the desktop host (web has no feed). Injected via `resolveSource` in tests.
export function loadNotificationSource(): Promise<NotificationSource | null> {
  if (!isDesktopHost()) return Promise.resolve(null);
  return Promise.resolve({
    history: () => commands.notificationsList().then(ensureResult),
    subscribe: (handler) => events.notificationEvent.listen((e) => handler(e.payload)),
  });
}
