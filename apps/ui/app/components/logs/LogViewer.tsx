import type { QueryClient } from "@tanstack/react-query";
import type { LogChannelHandle, LogsChangedChannelHandle } from "@tillerd/client-bindings";

import { queryOptions, useQuery, useQueryClient } from "@tanstack/react-query";
import { useVirtualizer } from "@tanstack/react-virtual";
import { query, logChannel, logsChangedChannel } from "@tillerd/client-bindings";
import React from "react";

import type { LogRecord } from "~/lib/logs/log-record";

import {
  LEVELS,
  type LogFilter,
  distinctAttribute,
  distinctService,
  filterRecords,
} from "~/lib/logs/log-filter";
import { parseRecord } from "~/lib/logs/log-record";
import { run } from "~/lib/subscribe";
import { isDesktopHost } from "~/lib/transport";
import { cn } from "~/lib/utils";

// Last window pulled per file on backlog; live tail then arrives via logChannel. "Load older"
// widens the window by one step.
const BACKFILL_BYTES = 256 * 1024;
const OLDER_STEP_BYTES = 256 * 1024;
const MAX_RECORDS = 10_000;

type LogRecordView = {
  timestamp: string;
  level: string;
  body: string;
  attributes: unknown;
  resource: unknown;
  raw: string;
};

function asRecord(value: unknown): Record<string, unknown> {
  return typeof value === "object" && value !== null ? (value as Record<string, unknown>) : {};
}

function fromView(view: LogRecordView): LogRecord {
  return {
    timestamp: view.timestamp,
    level: view.level,
    body: view.body,
    attributes: asRecord(view.attributes),
    resource: asRecord(view.resource),
    raw: view.raw,
  };
}

function sortByTime(records: LogRecord[]): LogRecord[] {
  return [...records].sort((a, b) =>
    a.timestamp < b.timestamp ? -1 : a.timestamp > b.timestamp ? 1 : 0,
  );
}

// Service prefix the log subscription keys on (the file-name stem before the first dot).
function servicePrefix(name: string): string {
  const dot = name.indexOf(".");
  return dot === -1 ? name : name.slice(0, dot);
}

function logBacklogQuery(qc: QueryClient, windowBytes: number, enabled: boolean) {
  return queryOptions({
    queryKey: ["logs", "backlog", windowBytes],
    enabled,
    queryFn: async (): Promise<LogRecord[]> => {
      const files = await qc.fetchQuery(query("logList"));
      const tails = await Promise.all(
        files.map((file) =>
          qc.fetchQuery(
            query("logTail", {
              path: file.path,
              from: Math.max(0, file.size - windowBytes),
              maxBytes: windowBytes,
              align: file.size > windowBytes,
            }),
          ),
        ),
      );
      const records = tails.flatMap((tail) => tail.records.map(fromView));
      return sortByTime(records);
    },
  });
}

async function startLiveTail(
  qc: QueryClient,
  cancelled: { current: boolean },
  handlesRef: React.RefObject<LogChannelHandle[]>,
  append: (record: LogRecord) => void,
): Promise<void> {
  const files = await qc.fetchQuery(query("logList"));
  if (cancelled.current) return;
  const services = [...new Set(files.map((file) => servicePrefix(file.name)))];
  const decoder = new TextDecoder();
  const handles = await Promise.all(
    services.map((service) =>
      logChannel({ service }, (bytes) => {
        const line = decoder.decode(bytes);
        const record = parseRecord(line);
        if (record) append(record);
      }),
    ),
  );
  if (cancelled.current) {
    for (const handle of handles) void handle.close();
    return;
  }
  handlesRef.current = handles;
}

// Watch for log-directory changes and invalidate the "logs" queries. Kept as a plain async helper
// (not a hook/component) so the await stays out of the render path; the effect drives it via run().
async function startLogsChangedWatch(
  qc: QueryClient,
  cancelled: { current: boolean },
  handleRef: React.RefObject<LogsChangedChannelHandle | undefined>,
): Promise<void> {
  const handle = await logsChangedChannel(() => {
    if (cancelled.current) return;
    void qc.invalidateQueries({ queryKey: ["logs"] });
  });
  if (cancelled.current) {
    void handle.close();
    return;
  }
  handleRef.current = handle;
}

export interface LogViewerProps {
  initialService?: string;
}

export function LogViewer({ initialService }: LogViewerProps) {
  const qc = useQueryClient();
  const desktop = isDesktopHost();
  const [windowBytes, setWindowBytes] = React.useState(BACKFILL_BYTES);
  const backlog = useQuery(logBacklogQuery(qc, windowBytes, desktop));
  // High-frequency-stream exception (client-engine spec: "A high-frequency stream renders from a
  // bounded local buffer"): live records append to a bounded local buffer merged at render time --
  // patching the Query cache per record would re-render every subscriber on every log line. The
  // durable half (backlog, file list) stays on the Query cache and revalidates by invalidation.
  const [live, setLive] = React.useState<LogRecord[]>([]);
  const [filter, setFilter] = React.useState<LogFilter>(() =>
    initialService ? { service: initialService } : {},
  );
  const [prevService, setPrevService] = React.useState(initialService);
  if (initialService !== prevService) {
    setPrevService(initialService);
    setFilter((f) => ({ ...f, service: initialService }));
  }
  const handlesRef = React.useRef<LogChannelHandle[]>([]);
  const changedRef = React.useRef<LogsChangedChannelHandle | undefined>(undefined);
  const scrollRef = React.useRef<HTMLDivElement>(null);
  const stickRef = React.useRef(true);

  const append = React.useCallback((record: LogRecord) => {
    setLive((rows) => {
      const next = rows.length >= MAX_RECORDS ? rows.slice(rows.length - MAX_RECORDS + 1) : rows;
      return [...next, record];
    });
  }, []);

  React.useEffect(() => {
    const cancelled = { current: false };
    run(startLogsChangedWatch(qc, cancelled, changedRef));
    return () => {
      cancelled.current = true;
      void changedRef.current?.close();
      changedRef.current = undefined;
    };
  }, [qc]);

  React.useEffect(() => {
    if (!desktop) return;
    const cancelled = { current: false };
    run(startLiveTail(qc, cancelled, handlesRef, append));
    return () => {
      cancelled.current = true;
      for (const handle of handlesRef.current) void handle.close();
      handlesRef.current = [];
    };
  }, [qc, append, desktop]);

  const handleLoadOlder = React.useCallback(() => {
    setWindowBytes((b) => b + OLDER_STEP_BYTES);
  }, []);

  const records = React.useMemo(
    () => sortByTime((backlog.data ?? []).concat(live)),
    [backlog.data, live],
  );

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

  if (!desktop) {
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
