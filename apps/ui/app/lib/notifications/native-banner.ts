import type { NotificationWire } from "@tillerd/client-bindings";

import { isDesktopHost } from "~/lib/transport/core";

import { notificationHeading } from "./store";

export interface BannerDeps {
  isFocused(): Promise<boolean>;
  isPermissionGranted(): Promise<boolean>;
  requestPermission(): Promise<boolean>;
  send(title: string, body: string): void;
}

export async function raiseBanner(event: NotificationWire, deps: BannerDeps): Promise<void> {
  if (await deps.isFocused()) return;
  if (!(await deps.isPermissionGranted())) {
    const granted = await deps.requestPermission();
    if (!granted) return;
  }
  deps.send(notificationHeading(event), event.message);
}

export async function loadBannerDeps(): Promise<BannerDeps | null> {
  try {
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
  } catch (err) {
    console.warn("loadBannerDeps failed, falling back to no native banners:", err);
    return null;
  }
}
