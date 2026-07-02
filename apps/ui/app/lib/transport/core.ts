export interface TauriChannelLike {
  onmessage: ((data: unknown) => void) | null;
}

export interface TauriCore {
  invoke<T = unknown>(cmd: string, args?: Record<string, unknown>): Promise<T>;
  createChannel(): TauriChannelLike;
  listen<T = unknown>(event: string, handler: (payload: T) => void): Promise<() => void>;
}

export function isDesktopHost(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function loadTauriCore(): Promise<TauriCore> {
  const { invoke, Channel } = await import("@tauri-apps/api/core");
  const { listen } = await import("@tauri-apps/api/event");
  return {
    invoke: (cmd, args) => invoke(cmd, args),
    createChannel: () => new Channel() as unknown as TauriChannelLike,
    listen: <T>(event: string, handler: (payload: T) => void) =>
      listen<T>(event, (e) => handler(e.payload)),
  };
}

export async function withDesktopCore<T>(
  build: (core: TauriCore) => T | Promise<T>,
): Promise<T | null> {
  if (!isDesktopHost()) return null;
  return build(await loadTauriCore());
}
