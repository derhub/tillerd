import { recordNotification } from "~/lib/notifications/context";

// Client-originated feedback (import/export results, "no active session" guards, ...): the
// notification center is the sole feedback channel (no toasts), so a client-side action that
// needs to tell the user something routes through the same NotificationWire shape the backend
// channel produces rather than inventing a second UI.
export function notify(category: string, severity: "info" | "error", message: string): void {
  recordNotification({
    id: crypto.randomUUID(),
    category,
    severity,
    title: null,
    message,
    detail: null,
    ts: Date.now(),
    sessionId: null,
    surfaceId: null,
  });
}
