import { useQuery } from "@tanstack/react-query";
import { query } from "@tillerd/client-bindings";
import React from "react";

import { Popover, PopoverContent, PopoverTrigger } from "~/components/ui/popover";
import { aggregateHealthState, type AggregateState } from "~/lib/health/aggregate";
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

export function ServiceHealthIndicator() {
  const host = useDesktopHost();
  const phase = host.status;
  // Plain useQuery, not Suspense: shell chrome degrades gracefully (renders the "starting"
  // aggregate with no data) instead of suspending; query data survives an error, so a failed
  // refetch keeps the prior snapshot.
  const health = useQuery({ ...query("serviceHealth"), enabled: phase !== "web" });
  const services = health.data ?? [];

  if (phase === "web") return null;

  const reason = host.status === "error" ? host.error.message : undefined;
  const aggregate = aggregateHealthState(phase, services);

  return (
    <Popover>
      <PopoverTrigger
        aria-label={`Service health: ${aggregate}`}
        className="flex items-center gap-1.5 rounded-sm px-2 h-6 font-mono text-[0.75rem] select-none text-muted-foreground hover:text-foreground hover:bg-muted transition-colors duration-[var(--motion-fast)] ease-standard focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
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
