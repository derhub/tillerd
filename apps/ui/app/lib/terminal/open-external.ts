import { isDesktopHost } from "~/lib/transport/core";

const SAFE_PROTOCOLS = new Set(["http:", "https:", "mailto:"]);

// Only browser-safe schemes reach the system opener -- an OSC 8 link or a mis-detected token could
// otherwise carry file: or javascript:. Mirrors the terminal link providers' http-only default.
export function isOpenableUrl(url: string): boolean {
  try {
    return SAFE_PROTOCOLS.has(new URL(url).protocol);
  } catch {
    return false;
  }
}

// Opens a terminal link in the system browser: the Tauri opener plugin on desktop, a new tab on
// the web host. Lives outside React (called with `void` from the link handler).
export async function openExternalUrl(url: string): Promise<void> {
  if (!isOpenableUrl(url)) return;
  try {
    if (isDesktopHost()) {
      const { openUrl } = await import("@tauri-apps/plugin-opener");
      await openUrl(url);
      return;
    }
    window.open(url, "_blank", "noopener,noreferrer");
  } catch (err) {
    console.warn("openExternalUrl failed:", err);
  }
}
