import { createContext, useContext, useEffect, useState, type ReactNode } from "react";
import { isDesktopHost } from "~/lib/transport";
import { createDesktopOrchestratorClient } from "~/lib/transport/orchestrator";
import { isFailed, isReady, type OrchestratorStatus } from "@tillerd/sdk/orchestrator";

export type DesktopHostState =
  | { status: "web" }
  | { status: "booting" }
  | { status: "ready" }
  | { status: "error"; error: Error };

const DesktopHostContext = createContext<DesktopHostState>({ status: "web" });

function toState(status: OrchestratorStatus): DesktopHostState {
  if (isReady(status)) return { status: "ready" };
  if (isFailed(status)) return { status: "error", error: new Error(status.reason) };
  return { status: "booting" };
}

/**
 * Observes the embedded orchestrator's readiness through the SDK client and
 * provides it to the tree. The renderer constructs no agent engine of its own
 * (ADR-0022): all backend interaction goes through the orchestrator API. On the
 * web deployment it is inert (`status: "web"`). A blank renderer that reaches
 * `ready` is the bar for this slice.
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
        const client = createDesktopOrchestratorClient();
        // Subscribe before the first read so no transition is missed; the
        // orchestrator never reports ready until its store is open and its
        // services are available.
        unlisten = await client.subscribe((status) => {
          if (!cancelled) setState(toState(status));
        });
        if (cancelled) {
          unlisten();
          return;
        }
        const current = await client.status();
        if (!cancelled) setState(toState(current));
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
