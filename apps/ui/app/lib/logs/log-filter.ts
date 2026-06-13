import type { LogRecord } from "./log-record";

export interface LogFilter {
  /** Exact severity level; only records of that level are shown. */
  level?: string;
  /** Free-text query over the body and attributes. */
  query?: string;
  /** `component` attribute facet. */
  component?: string;
  /** `session.id` attribute facet. */
  sessionId?: string;
  /** `service.name` resource facet; lets a health row deep-link to one service's logs. */
  service?: string;
}

/** Severity levels, low to high. Single source of truth for level ordering. */
export const LEVELS = ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"] as const;

/** Apply a {@link LogFilter} to records. An empty filter returns them unchanged. */
export function filterRecords(records: LogRecord[], filter: LogFilter): LogRecord[] {
  const level = filter.level?.toUpperCase();
  const query = filter.query?.trim().toLowerCase() ?? "";
  return records.filter((r) => {
    if (level && r.level.toUpperCase() !== level) return false;
    if (filter.component && r.attributes.component !== filter.component) return false;
    if (filter.sessionId && r.attributes["session.id"] !== filter.sessionId) return false;
    if (filter.service && r.resource["service.name"] !== filter.service) return false;
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

/** Sorted distinct `service.name` resource values across records (for the service facet). */
export function distinctService(records: LogRecord[]): string[] {
  const values = new Set<string>();
  for (const r of records) {
    const v = r.resource["service.name"];
    if (typeof v === "string") values.add(v);
  }
  return [...values].sort();
}
