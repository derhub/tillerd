import { createContext, useContext, useEffect, useState, type ReactNode } from "react";
import {
  isDesktopHost,
  loadTauriCore,
  bootDesktopHost,
  type DesktopHost,
  type TauriCore,
} from "~/lib/transport";

export type DesktopHostState =
  | { status: "web" }
  | { status: "booting" }
  | { status: "ready"; host: DesktopHost; core: TauriCore }
  | { status: "error"; error: Error };

const DesktopHostContext = createContext<DesktopHostState>({ status: "web" });

/**
 * Boots the desktop host once (resolve agent + ensure daemon + construct engine) and provides it
 * to the tree. On the web deployment it is inert — `status: "web"` — and components keep their
 * existing network behavior.
 */
export function DesktopHostProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<DesktopHostState>(() =>
    isDesktopHost() ? { status: "booting" } : { status: "web" },
  );

  useEffect(() => {
    if (!isDesktopHost()) return;
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    void (async () => {
      try {
        const core = await loadTauriCore();
        const host = await bootDesktopHost(core);
        if (cancelled) return;
        // §5.6: the native core emits `daemon-lost` on an unexpected daemon exit.
        const { listen } = await import("@tauri-apps/api/event");
        unlisten = await listen("daemon-lost", () => {
          setState({ status: "error", error: new Error("daemon connection lost") });
        });
        if (cancelled) {
          unlisten();
          return;
        }
        setState({ status: "ready", host, core });
      } catch (e) {
        if (!cancelled) {
          setState({ status: "error", error: e instanceof Error ? e : new Error(String(e)) });
        }
      }
    })();
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  return <DesktopHostContext.Provider value={state}>{children}</DesktopHostContext.Provider>;
}

export function useDesktopHost(): DesktopHostState {
  return useContext(DesktopHostContext);
}
