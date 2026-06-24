import type { ServiceHealthWire } from "@tillerd/client-bindings";

import React from "react";

import { Popover, PopoverContent, PopoverTrigger } from "~/components/ui/popover";
import { aggregateHealthState, type AggregateState } from "~/lib/health/aggregate";
import {
  loadServiceHealthSource,
  type ServiceHealthSource,
} from "~/lib/transport/service-health-source";
import { useDesktopHost } from "~/lib/useDesktopHost";
import { cn } from "~/lib/utils";

import { ServiceHealthPanel } from "./ServiceHealthPanel";

async function fetchHealthSnapshot(
  resolveSource: () => Promise<ServiceHealthSource | null>,
  cancelled: { current: boolean },
  setServices: (s: ServiceHealthWire[]) => void,
): Promise<void> {
  const source = await resolveSource();
  if (!source || cancelled.current) return;
  try {
    const snapshot = await source.snapshot();
    if (!cancelled.current) setServices(snapshot);
  } catch {
    // Keep the prior snapshot; indicator degrades gracefully on fetch failure.
  }
}

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
  resolveSource?: () => Promise<ServiceHealthSource | null>;
}

export function ServiceHealthIndicator({
  resolveSource = loadServiceHealthSource,
}: ServiceHealthIndicatorProps = {}) {
  const host = useDesktopHost();
  const phase = host.status;
  const [services, setServices] = React.useState<ServiceHealthWire[]>([]);

  React.useEffect(() => {
    if (phase === "web") return;
    const cancelled = { current: false };
    void fetchHealthSnapshot(resolveSource, cancelled, setServices);
    return () => {
      cancelled.current = true;
    };
  }, [phase, resolveSource]);

  if (phase === "web") return null;

  const reason = host.status === "error" ? host.error.message : undefined;
  const aggregate = aggregateHealthState(phase, services);

  return (
    <Popover>
      <PopoverTrigger
        aria-label={`Service health: ${aggregate}`}
        className="flex items-center gap-1.5 rounded-sm bg-black/60 px-2 h-6 font-mono text-[0.75rem] select-none"
      >
        <span className={cn("w-1.5 h-1.5 rounded-full", DOT[aggregate])} />
        <span className={TEXT[aggregate]}>services: {aggregate}</span>
      </PopoverTrigger>
      <PopoverContent>
        <ServiceHealthPanel phase={phase} reason={reason} services={services} />
      </PopoverContent>
    </Popover>
  );
}
