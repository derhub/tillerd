import { useVirtualizer } from "@tanstack/react-virtual";
import React from "react";

import type { LogRecord } from "~/lib/logs/log-record";

import {
  LEVELS,
  type LogFilter,
  distinctAttribute,
  distinctService,
  filterRecords,
} from "~/lib/logs/log-filter";
import { LogTail } from "~/lib/logs/log-tail";
import { type LogSource, loadLogSource } from "~/lib/transport/log-source";
import { cn } from "~/lib/utils";

async function startLogTail(
  resolveSource: () => Promise<LogSource | null>,
  pollMs: number,
  cancelled: { current: boolean },
  tailRef: React.RefObject<LogTail | null>,
  timerRef: { current: ReturnType<typeof setInterval> | undefined },
  busyRef: React.RefObject<boolean>,
  lastRef: React.RefObject<LogRecord[] | null>,
  setUnsupported: (v: boolean) => void,
  setRecords: (v: LogRecord[]) => void,
): Promise<void> {
  const source = await resolveSource();
  if (cancelled.current) return;
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
      if (!cancelled.current && next !== lastRef.current) {
        lastRef.current = next;
        setRecords([...next]);
      }
    } finally {
      busyRef.current = false;
    }
  };
  await tick();
  if (!cancelled.current) timerRef.current = setInterval(() => void tick(), pollMs);
}

async function loadOlderRecords(
  tail: LogTail,
  busyRef: React.RefObject<boolean>,
  lastRef: React.RefObject<LogRecord[] | null>,
  setRecords: (v: LogRecord[]) => void,
): Promise<void> {
  if (busyRef.current) return;
  busyRef.current = true;
  try {
    const next = await tail.loadOlderAll();
    lastRef.current = next;
    setRecords([...next]);
  } finally {
    busyRef.current = false;
  }
}

const POLL_MS = 1000;

export interface LogViewerProps {
  resolveSource?: () => Promise<LogSource | null>;
  pollMs?: number;
  initialService?: string;
}

export function LogViewer({
  resolveSource = loadLogSource,
  pollMs = POLL_MS,
  initialService,
}: LogViewerProps) {
  const [records, setRecords] = React.useState<LogRecord[]>([]);
  const [unsupported, setUnsupported] = React.useState(false);
  const [filter, setFilter] = React.useState<LogFilter>(() =>
    initialService ? { service: initialService } : {},
  );
  const [prevService, setPrevService] = React.useState(initialService);
  if (initialService !== prevService) {
    setPrevService(initialService);
    setFilter((f) => ({ ...f, service: initialService }));
  }
  const tailRef = React.useRef<LogTail | null>(null);
  const busyRef = React.useRef(false);
  const lastRef = React.useRef<LogRecord[] | null>(null);
  const scrollRef = React.useRef<HTMLDivElement>(null);
  const stickRef = React.useRef(true);

  React.useEffect(() => {
    const cancelled = { current: false };
    const timerRef = { current: undefined as ReturnType<typeof setInterval> | undefined };
    void startLogTail(
      resolveSource,
      pollMs,
      cancelled,
      tailRef,
      timerRef,
      busyRef,
      lastRef,
      setUnsupported,
      setRecords,
    );
    return () => {
      cancelled.current = true;
      if (timerRef.current) clearInterval(timerRef.current);
    };
  }, [resolveSource, pollMs]);

  const handleLoadOlder = React.useCallback(() => {
    const tail = tailRef.current;
    if (!tail) return;
    void loadOlderRecords(tail, busyRef, lastRef, setRecords);
  }, []);

  const components = React.useMemo(() => distinctAttribute(records, "component"), [records]);
  const sessions = React.useMemo(() => distinctAttribute(records, "session.id"), [records]);
  const serviceNames = React.useMemo(() => distinctService(records), [records]);
  const shown = React.useMemo(() => filterRecords(records, filter), [records, filter]);

  const virtualizer = useVirtualizer({
    count: shown.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => 22,
    overscan: 24,
  });

  React.useEffect(() => {
    if (stickRef.current && shown.length > 0) {
      virtualizer.scrollToIndex(shown.length - 1, { align: "end" });
    }
  }, [shown, virtualizer]);

  const onScroll = React.useCallback(() => {
    const el = scrollRef.current;
    if (el) stickRef.current = el.scrollHeight - el.scrollTop - el.clientHeight < 16;
  }, []);

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
          onClick={() => handleLoadOlder()}
          className="border border-border/40 rounded-sm px-1.5 py-0.5 text-muted-foreground hover:text-foreground hover:bg-muted"
        >
          load older
        </button>
        <select
          aria-label="Level"
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
          label="service"
          values={serviceNames}
          value={filter.service}
          onChange={(v) => setFilter((f) => ({ ...f, service: v }))}
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
      <div
        ref={scrollRef}
        onScroll={onScroll}
        data-testid="log-scroll"
        className="flex-1 overflow-auto"
      >
        <div style={{ height: `${virtualizer.getTotalSize()}px`, position: "relative" }}>
          {virtualizer.getVirtualItems().map((vi) => (
            <div
              key={vi.key}
              data-index={vi.index}
              ref={virtualizer.measureElement}
              style={{
                position: "absolute",
                top: 0,
                left: 0,
                width: "100%",
                transform: `translateY(${vi.start}px)`,
              }}
            >
              <LogRow record={shown[vi.index]} />
            </div>
          ))}
        </div>
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

const LogRow = React.memo(function LogRow({ record }: { record: LogRecord }) {
  const service = strAttr(record.resource, "service.name");
  const session = strAttr(record.attributes, "session.id");
  return (
    <div
      data-service={service}
      className="flex gap-2 px-3 py-0.5 border-b border-border/10 whitespace-pre-wrap break-all"
    >
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
