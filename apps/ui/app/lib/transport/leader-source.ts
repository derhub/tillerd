import { isDesktopHost, loadTauriCore } from "./core";

/** Event the desktop host emits when the native leader accelerator fires. */
export const COMMAND_CENTER_OPEN_EVENT = "command-center:open";
/** Command that updates the native leader accelerator. */
export const SET_LEADER_COMMAND = "command_center_set_leader";

/**
 * Host-agnostic leader-key activation. The desktop adapter registers a native menu accelerator
 * (so it fires over terminal focus) and surfaces it as an event; a future server/web adapter
 * satisfies the same contract with a document-level key listener. `null` off the desktop host.
 */
export interface LeaderKeyPort {
  /** Subscribe to leader activation. Resolves to an unsubscribe. */
  onActivate(handler: () => void): Promise<() => void>;
  /** Update the leader accelerator on the host. */
  setBinding(accelerator: string): Promise<void>;
}

export async function loadLeaderKeyPort(): Promise<LeaderKeyPort | null> {
  if (!isDesktopHost()) return null;
  const core = await loadTauriCore();
  return {
    onActivate: async (handler) => {
      const { listen } = await import("@tauri-apps/api/event");
      return listen(COMMAND_CENTER_OPEN_EVENT, () => handler());
    },
    setBinding: async (accelerator) => {
      await core.invoke(SET_LEADER_COMMAND, { accelerator });
    },
  };
}
