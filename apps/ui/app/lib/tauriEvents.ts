// Single boundary for Tauri event + window APIs. Centralises the isDesktopHost() guard and dynamic imports so callers need no host check; every call is a no-op (or null) off the desktop host.

import { isDesktopHost } from "./transport/core";

export async function emitEvent(event: string, payload: unknown): Promise<void> {
  if (!isDesktopHost()) return;
  const { emit } = await import("@tauri-apps/api/event");
  await emit(event, payload);
}

export async function listenEvent<T>(
  event: string,
  handler: (payload: T) => void,
): Promise<() => void> {
  if (!isDesktopHost()) return () => {};
  const { listen } = await import("@tauri-apps/api/event");
  return listen<T>(event, (e) => handler(e.payload));
}

export async function currentWindow() {
  if (!isDesktopHost()) return null;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  return getCurrentWindow();
}

// Label is stable for the process lifetime; resolve the dynamic import once.
let cachedLabel: string | null = null;
export async function windowLabel(): Promise<string | null> {
  if (cachedLabel !== null) return cachedLabel;
  const win = await currentWindow();
  if (!win) return null;
  cachedLabel = win.label;
  return cachedLabel;
}
