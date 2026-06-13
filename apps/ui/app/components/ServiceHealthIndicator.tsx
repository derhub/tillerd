import { useEffect, useState } from "react";
import { useNavigate } from "react-router";
import type { ServiceHealth } from "@tillerd/sdk/orchestrator";

import { Popover, PopoverContent, PopoverTrigger } from "~/components/ui/popover";
import { aggregateHealthState, type AggregateState } from "~/lib/health/aggregate";
import {
  loadServiceHealthSource,
  type ServiceHealthSource,
} from "~/lib/transport/service-health-source";
import { useDesktopHost } from "~/lib/useDesktopHost";
import { cn } from "~/lib/utils";

import { ServiceHealthPanel } from "./ServiceHealthPanel";

const DOT: Record<AggregateState, string> = {
  ready: "bg-emerald-500",
  starting: "bg-amber-500",
  failed: "bg-red-500",
};

const TEXT: Record<AggregateState, string> = {
  ready: "text-emerald-300",
  starting: "text-amber-300",
  failed: "text-red-300",
};

export interface ServiceHealthIndicatorProps {
  /** Override the source resolver; tests inject a fake. Defaults to the host adapter. */
  resolveSource?: () => Promise<ServiceHealthSource | null>;
}

/**
 * Single read-only health indicator in the shell's bottom-right cluster. Its dot
 * shows the worst state across the orchestrator and services; clicking it opens a
 * panel listing each one. Hidden off the desktop host. The snapshot re-reads on
 * each orchestrator phase change (the existing status event) — there is no health
 * event of its own.
 */
export function ServiceHealthIndicator({
  resolveSource = loadServiceHealthSource,
}: ServiceHealthIndicatorProps = {}) {
  const host = useDesktopHost();
  const navigate = useNavigate();
  const phase = host.status;
  const [services, setServices] = useState<ServiceHealth[]>([]);

  useEffect(() => {
    if (phase === "web") return;
    let cancelled = false;
    void (async () => {
      const source = await resolveSource();
      if (!source || cancelled) return;
      try {
        const snapshot = await source.snapshot();
        if (!cancelled) setServices(snapshot);
      } catch {
        // Keep the prior snapshot; the indicator degrades to what it last knew.
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [phase, resolveSource]);

  if (phase === "web") return null;

  const reason = host.status === "error" ? host.error.message : undefined;
  const aggregate = aggregateHealthState(phase, services);

  return (
    <div className="fixed bottom-2 right-2 z-50">
      <Popover>
        <PopoverTrigger
          aria-label={`Service health: ${aggregate}`}
          className="flex items-center gap-1.5 rounded-sm bg-black/60 px-2 h-6 font-mono text-[0.75rem] select-none"
        >
          <span className={cn("w-1.5 h-1.5 rounded-full", DOT[aggregate])} />
          <span className={TEXT[aggregate]}>services: {aggregate}</span>
        </PopoverTrigger>
        <PopoverContent>
          <ServiceHealthPanel
            phase={phase}
            reason={reason}
            services={services}
            onOpenLogs={(service) => void navigate(`/logs?service=${encodeURIComponent(service)}`)}
          />
        </PopoverContent>
      </Popover>
    </div>
  );
}
