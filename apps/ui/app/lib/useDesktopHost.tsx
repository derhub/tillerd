import { createContext, useContext, useEffect, useState, type ReactNode } from "react";
import { isDesktopHost } from "~/lib/transport";
import { createDesktopOrchestratorClient } from "~/lib/transport/orchestrator";
import {
  isFailed,
  isReady,
  type OrchestratorClient,
  type OrchestratorStatus,
} from "@tillerd/sdk/orchestrator";

export type DesktopHostState =
  | { status: "web" }
  | { status: "booting" }
  | { status: "ready"; orchestratorClient: OrchestratorClient }
  | { status: "error"; error: Error };

const DesktopHostContext = createContext<DesktopHostState>({ status: "web" });

function toState(status: OrchestratorStatus, client: OrchestratorClient): DesktopHostState {
  if (isReady(status)) return { status: "ready", orchestratorClient: client };
  if (isFailed(status)) return { status: "error", error: new Error(status.reason) };
  return { status: "booting" };
}

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
        const client = createDesktopOrchestratorClient();
        // subscribe before the first read so no transition is missed
        unlisten = await client.subscribe((status) => {
          if (!cancelled) setState(toState(status, client));
        });
        if (cancelled) {
          unlisten();
          return;
        }
        const current = await client.status();
        if (!cancelled) setState(toState(current, client));
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
