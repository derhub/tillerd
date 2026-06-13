import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";

import { LEVELS, type LogFilter, distinctAttribute, filterRecords } from "~/lib/logs/log-filter";
import type { LogRecord } from "~/lib/logs/log-record";
import { LogTail } from "~/lib/logs/log-tail";
import { type LogSource, loadLogSource } from "~/lib/transport/log-source";
import { cn } from "~/lib/utils";

const POLL_MS = 1000;

export interface LogViewerProps {
  /** Override the source resolver; tests inject a fake. Defaults to the host adapter. */
  resolveSource?: () => Promise<LogSource | null>;
  pollMs?: number;
}

/**
 * Global log viewer: tails every service's structured log file through a host
 * {@link LogSource} and renders the merged records with level, text, and facet
 * filtering. App-shell chrome, not a session surface. Off the desktop host the
 * source is `null` (the server adapter is deferred) and a notice is shown.
 */
export function LogViewer({ resolveSource = loadLogSource, pollMs = POLL_MS }: LogViewerProps) {
  const [records, setRecords] = useState<LogRecord[]>([]);
  const [unsupported, setUnsupported] = useState(false);
  const [filter, setFilter] = useState<LogFilter>({});
  const tailRef = useRef<LogTail | null>(null);
  // Serializes refresh ticks and load-older against each other so they never
  // mutate the shared LogTail concurrently.
  const busyRef = useRef(false);
  // LogTail returns the same array reference when a refresh adds nothing; skip the
  // state update (and the row re-render) in that case.
  const lastRef = useRef<LogRecord[] | null>(null);

  useEffect(() => {
    let cancelled = false;
    let timer: ReturnType<typeof setInterval> | undefined;
    void (async () => {
      const source = await resolveSource();
      if (cancelled) return;
      if (!source) {
        setUnsupported(true);
        return;
      }
      const tail = new LogTail(source);
      tailRef.current = tail;
      const tick = async () => {
        if (busyRef.current) return;
        busyRef.current = true;
        try {
          const next = await tail.refresh();
          if (!cancelled && next !== lastRef.current) {
            lastRef.current = next;
            setRecords([...next]);
          }
        } finally {
          busyRef.current = false;
        }
      };
      await tick();
      timer = setInterval(() => void tick(), pollMs);
    })();
    return () => {
      cancelled = true;
      if (timer) clearInterval(timer);
    };
  }, [resolveSource, pollMs]);

  const handleLoadOlder = useCallback(async () => {
    const tail = tailRef.current;
    if (!tail || busyRef.current) return;
    busyRef.current = true;
    try {
      const next = await tail.loadOlderAll();
      lastRef.current = next;
      setRecords([...next]);
    } finally {
      busyRef.current = false;
    }
  }, []);

  const components = useMemo(() => distinctAttribute(records, "component"), [records]);
  const sessions = useMemo(() => distinctAttribute(records, "session.id"), [records]);
  const shown = useMemo(() => filterRecords(records, filter), [records, filter]);

  if (unsupported) {
    return (
      <div className="h-full flex items-center justify-center text-xs text-muted-foreground">
        Log viewer is available on the desktop app.
      </div>
    );
  }

  return (
    <div data-testid="log-viewer" className="h-full flex flex-col text-xs font-mono">
      <div className="flex items-center gap-2 px-3 h-9 border-b border-border/40 shrink-0">
        <button
          type="button"
          onClick={() => void handleLoadOlder()}
          className="border border-border/40 rounded-sm px-1.5 py-0.5 text-muted-foreground hover:text-foreground hover:bg-muted"
        >
          load older
        </button>
        <select
          aria-label="Minimum level"
          className="bg-transparent border border-border/40 rounded-sm px-1 py-0.5"
          value={filter.level ?? ""}
          onChange={(e) => setFilter((f) => ({ ...f, level: e.target.value || undefined }))}
        >
          <option value="">all levels</option>
          {LEVELS.map((l) => (
            <option key={l} value={l}>
              {l}
            </option>
          ))}
        </select>
        <input
          aria-label="Search logs"
          className="bg-transparent border border-border/40 rounded-sm px-1 py-0.5 flex-1 min-w-0"
          placeholder="search"
          value={filter.query ?? ""}
          onChange={(e) => setFilter((f) => ({ ...f, query: e.target.value || undefined }))}
        />
        <Facet
          label="component"
          values={components}
          value={filter.component}
          onChange={(v) => setFilter((f) => ({ ...f, component: v }))}
        />
        <Facet
          label="session"
          values={sessions}
          value={filter.sessionId}
          onChange={(v) => setFilter((f) => ({ ...f, sessionId: v }))}
        />
      </div>
      <div className="flex-1 overflow-auto">
        {shown.map((r, i) => (
          <LogRow key={`${r.timestamp}-${i}`} record={r} />
        ))}
      </div>
    </div>
  );
}

function Facet({
  label,
  values,
  value,
  onChange,
}: {
  label: string;
  values: string[];
  value?: string;
  onChange: (v?: string) => void;
}) {
  return (
    <select
      aria-label={label}
      className="bg-transparent border border-border/40 rounded-sm px-1 py-0.5"
      value={value ?? ""}
      onChange={(e) => onChange(e.target.value || undefined)}
    >
      <option value="">all {label}</option>
      {values.map((v) => (
        <option key={v} value={v}>
          {v}
        </option>
      ))}
    </select>
  );
}

const LEVEL_COLOR: Record<string, string> = {
  ERROR: "text-red-400",
  WARN: "text-amber-400",
  INFO: "text-emerald-400",
  DEBUG: "text-sky-400",
  TRACE: "text-muted-foreground",
};

function strAttr(obj: Record<string, unknown>, key: string): string {
  return typeof obj[key] === "string" ? (obj[key] as string) : "";
}

const LogRow = memo(function LogRow({ record }: { record: LogRecord }) {
  const service = strAttr(record.resource, "service.name");
  const session = strAttr(record.attributes, "session.id");
  return (
    <div className="flex gap-2 px-3 py-0.5 border-b border-border/10 whitespace-pre-wrap break-all">
      <span className="text-muted-foreground shrink-0">{record.timestamp}</span>
      <span className={cn("shrink-0 w-12", LEVEL_COLOR[record.level.toUpperCase()] ?? "")}>
        {record.level}
      </span>
      <span className="text-muted-foreground shrink-0">{service}</span>
      {session ? <span className="text-muted-foreground/70 shrink-0">{session}</span> : null}
      <span className="flex-1 min-w-0">{record.body}</span>
    </div>
  );
});
