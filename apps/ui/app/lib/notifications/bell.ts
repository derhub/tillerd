import type { NotificationWire } from "@tillerd/client-bindings";

import { recordNotification } from "./context";
import { loadBannerDeps, raiseBanner, type BannerDeps } from "./native-banner";

export interface BellContext {
  sessionId: string | null;
  surfaceId: string | null;
  sessionLabel?: string | null;
  now?: number;
  id?: string;
}

// A terminal bell as a notification-center event (ui-terminal-pane "Bell surfaces as a
// notification"). Pure so the wire shape is verified without a live terminal; attribution rides on
// sessionId/surfaceId and, when a session label is at hand, the message names it.
export function buildBellNotification(ctx: BellContext): NotificationWire {
  const message = ctx.sessionLabel ? `Bell in "${ctx.sessionLabel}"` : "A terminal rang the bell.";
  return {
    id: ctx.id ?? crypto.randomUUID(),
    category: "surface-bell",
    severity: "info",
    title: null,
    message,
    detail: null,
    ts: ctx.now ?? Date.now(),
    sessionId: ctx.sessionId,
    surfaceId: ctx.surfaceId,
  };
}

// Record the bell in the notification center and, when the window is unfocused, raise the native
// banner (raiseBanner owns the focus gate). Lives outside React so the pane fires it with `void`
// -- no async function inside a component.
export async function emitBellNotification(
  ctx: BellContext,
  resolveBanner: () => Promise<BannerDeps | null> = loadBannerDeps,
): Promise<void> {
  const event = buildBellNotification(ctx);
  recordNotification(event);
  const banner = await resolveBanner();
  if (banner) await raiseBanner(event, banner);
}
