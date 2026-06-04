import type { TauriCore } from "./tauri";

export const PREF_GET = "pref_get";
export const PREF_SET = "pref_set";
export const REGISTRY_GET = "registry_get";
export const REGISTRY_SET = "registry_set";
export const REGISTRY_REMOVE = "registry_remove";
export const REGISTRY_LIST = "registry_list";

export interface RegistryEntry {
  sessionId: string;
  cwd: string;
}

/**
 * Native app-data accessor: user preferences plus the session registry (sessionId -> cwd, used to
 * supply `cwd` on reconnect). Backed by the Rust core's local store over `invoke`; replaces the
 * server-side sqlite registry on the desktop path (design D6).
 */
export class TauriAppData {
  constructor(private readonly core: TauriCore) {}

  getPref<T = unknown>(key: string): Promise<T | null> {
    return this.core.invoke<T | null>(PREF_GET, { key });
  }

  setPref(key: string, value: unknown): Promise<void> {
    return this.core.invoke<void>(PREF_SET, { key, value });
  }

  /** cwd recorded for a session, or `null` when unknown. */
  getCwd(sessionId: string): Promise<string | null> {
    return this.core.invoke<string | null>(REGISTRY_GET, { sessionId });
  }

  recordSession(sessionId: string, cwd: string): Promise<void> {
    return this.core.invoke<void>(REGISTRY_SET, { sessionId, cwd });
  }

  removeSession(sessionId: string): Promise<void> {
    return this.core.invoke<void>(REGISTRY_REMOVE, { sessionId });
  }

  listSessions(): Promise<RegistryEntry[]> {
    return this.core.invoke<RegistryEntry[]>(REGISTRY_LIST);
  }

  /** Drop registry entries whose session is no longer live in the daemon. */
  async reconcile(liveIds: string[]): Promise<void> {
    const live = new Set(liveIds);
    const known = await this.listSessions();
    await Promise.all(
      known.filter((e) => !live.has(e.sessionId)).map((e) => this.removeSession(e.sessionId)),
    );
  }
}
