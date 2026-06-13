import { Link } from "react-router";
import type { ServiceHealth, ServiceState } from "@tillerd/sdk/orchestrator";

import type { OrchestratorPhase } from "~/lib/health/aggregate";
import { cn } from "~/lib/utils";

/** A row's state covers every service state plus the orchestrator's own `failed`. */
type RowState = ServiceState | "failed";

const STATE_LABEL: Record<RowState, string> = {
  ready: "ready",
  starting: "starting",
  draining: "draining",
  versionMismatch: "version mismatch",
  unavailable: "unavailable",
  failed: "failed",
};

const STATE_DOT: Record<RowState, string> = {
  ready: "bg-emerald-500",
  starting: "bg-amber-500",
  draining: "bg-amber-500",
  versionMismatch: "bg-orange-500",
  unavailable: "bg-red-500",
  failed: "bg-red-500",
};

function phaseToState(phase: OrchestratorPhase): RowState {
  if (phase === "ready") return "ready";
  if (phase === "error") return "failed";
  return "starting"; // booting; `web` never reaches the panel
}

/** Strip the `tillerd-` prefix for a compact label; the full name still drives the logs link. */
function shortLabel(serviceName: string): string {
  return serviceName.replace(/^tillerd-/, "");
}

function HealthRow({
  name,
  state,
  version,
  reason,
  logsService,
}: {
  name: string;
  state: RowState;
  version: string | null;
  reason?: string;
  logsService: string;
}) {
  return (
    <div className="flex items-center gap-2 px-2 py-1.5 text-xs">
      <span className={cn("w-1.5 h-1.5 rounded-full shrink-0", STATE_DOT[state])} />
      <span className="font-medium">{name}</span>
      <span className="text-muted-foreground">{version ?? "—"}</span>
      <span className="text-muted-foreground">{STATE_LABEL[state]}</span>
      {reason ? (
        <span className="text-red-300/70 truncate max-w-[16ch]" title={reason}>
          {reason}
        </span>
      ) : null}
      <Link
        to={`/logs?service=${encodeURIComponent(logsService)}`}
        className="ml-auto text-muted-foreground underline hover:text-foreground"
      >
        logs
      </Link>
    </div>
  );
}

export interface ServiceHealthPanelProps {
  phase: OrchestratorPhase;
  reason?: string;
  services: ServiceHealth[];
}

/**
 * Read-only health panel: one row per service plus the orchestrator, each with
 * version, state, an optional failure reason, and a logs link filtered to that
 * service. No control changes a service's lifecycle.
 */
export function ServiceHealthPanel({ phase, reason, services }: ServiceHealthPanelProps) {
  return (
    <div className="flex flex-col font-mono">
      <HealthRow
        name="orchestrator"
        state={phaseToState(phase)}
        version={null}
        reason={reason}
        logsService="tillerd-desktop"
      />
      {services.map((s) => (
        <HealthRow
          key={s.name}
          name={shortLabel(s.name)}
          state={s.state}
          version={s.version}
          logsService={s.name}
        />
      ))}
    </div>
  );
}
