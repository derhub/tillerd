import type { TauriCore, TauriChannelLike } from "./tauri";

/** Desktop host = a Tauri v2 web view, which injects `__TAURI_INTERNALS__` on `window`. */
export function isDesktopHost(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/** Bind the Tauri core surface; only reached on the desktop host. */
export async function loadTauriCore(): Promise<TauriCore> {
  const { invoke, Channel } = await import("@tauri-apps/api/core");
  return {
    invoke: (cmd, args) => invoke(cmd, args),
    createChannel: () => new Channel() as unknown as TauriChannelLike,
  };
}
