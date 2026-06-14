import type { NotificationEvent } from "@tillerd/sdk/orchestrator";

import { isDesktopHost } from "~/lib/transport/core";
import { notificationHeading } from "./store";

/** The host capabilities a native banner needs; injected so the policy is testable. */
export interface BannerDeps {
  /** Whether the app window currently has focus. */
  isFocused(): Promise<boolean>;
  /** Whether the OS notification permission is already granted. */
  isPermissionGranted(): Promise<boolean>;
  /** Prompt for permission; resolves to whether it was granted. */
  requestPermission(): Promise<boolean>;
  /** Send a native OS banner. */
  send(title: string, body: string): void;
}

/**
 * Raise a native OS banner for a background event. No-op when the window is focused (the in-app
 * feed covers it) or when the OS permission is denied — the feed still records it either way.
 */
export async function raiseBanner(event: NotificationEvent, deps: BannerDeps): Promise<void> {
  if (await deps.isFocused()) return;
  if (!(await deps.isPermissionGranted())) {
    const granted = await deps.requestPermission();
    if (!granted) return;
  }
  deps.send(notificationHeading(event), event.message);
}

/**
 * Bind the desktop {@link BannerDeps} (Tauri window focus + notification plugin). Returns `null`
 * off the desktop host — no native banners there.
 */
export async function loadBannerDeps(): Promise<BannerDeps | null> {
  if (!isDesktopHost()) return null;
  const [{ getCurrentWindow }, plugin] = await Promise.all([
    import("@tauri-apps/api/window"),
    import("@tauri-apps/plugin-notification"),
  ]);
  return {
    isFocused: () => getCurrentWindow().isFocused(),
    isPermissionGranted: () => plugin.isPermissionGranted(),
    requestPermission: async () => (await plugin.requestPermission()) === "granted",
    send: (title, body) => plugin.sendNotification({ title, body }),
  };
}
