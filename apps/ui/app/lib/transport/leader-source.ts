import { withDesktopCore } from "./core";

export const COMMAND_CENTER_OPEN_EVENT = "command-center:open";
export const SET_LEADER_COMMAND = "command_center_set_leader";

// Desktop adapter: native menu accelerator fires over terminal focus, surfaces as event.
// Web adapter satisfies the same contract with a document-level key listener.
// `null` off the desktop host.
export interface LeaderKeyPort {
  onActivate(handler: () => void): Promise<() => void>;
  setBinding(accelerator: string): Promise<void>;
}

export function loadLeaderKeyPort(): Promise<LeaderKeyPort | null> {
  return withDesktopCore((core) => ({
    onActivate: (handler) => core.listen(COMMAND_CENTER_OPEN_EVENT, () => handler()),
    setBinding: async (accelerator) => {
      await core.invoke(SET_LEADER_COMMAND, { accelerator });
    },
  }));
}
