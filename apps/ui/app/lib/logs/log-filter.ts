import type { LogRecord } from "./log-record";

export interface LogFilter {
  /** Minimum severity level; records below it are hidden. */
  level?: string;
  /** Free-text query over the body and attributes. */
  query?: string;
  /** `component` attribute facet. */
  component?: string;
  /** `session.id` attribute facet. */
  sessionId?: string;
}

/** Severity levels, low to high. Single source of truth for level ordering. */
export const LEVELS = ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"] as const;

const LEVEL_RANK: Record<string, number> = Object.fromEntries(LEVELS.map((l, i) => [l, i]));

function rank(level: string): number {
  return LEVEL_RANK[level.toUpperCase()] ?? 0;
}

/** Apply a {@link LogFilter} to records. An empty filter returns them unchanged. */
export function filterRecords(records: LogRecord[], filter: LogFilter): LogRecord[] {
  const min = filter.level ? rank(filter.level) : -1;
  const query = filter.query?.trim().toLowerCase() ?? "";
  return records.filter((r) => {
    if (min >= 0 && rank(r.level) < min) return false;
    if (filter.component && r.attributes.component !== filter.component) return false;
    if (filter.sessionId && r.attributes["session.id"] !== filter.sessionId) return false;
    if (query) {
      const haystack = `${r.body} ${JSON.stringify(r.attributes)}`.toLowerCase();
      if (!haystack.includes(query)) return false;
    }
    return true;
  });
}

/** Sorted distinct values of an attribute across records (for facet menus). */
export function distinctAttribute(records: LogRecord[], key: string): string[] {
  const values = new Set<string>();
  for (const r of records) {
    const v = r.attributes[key];
    if (typeof v === "string") values.add(v);
  }
  return [...values].sort();
}
