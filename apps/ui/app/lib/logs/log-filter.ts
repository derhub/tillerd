import type { LogRecord } from "./log-record";

export interface LogFilter {
  level?: string;
  query?: string;
  component?: string;
  sessionId?: string;
  service?: string;
}

export const LEVELS = ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"] as const;

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

export function distinctAttribute(records: LogRecord[], key: string): string[] {
  const values = new Set<string>();
  for (const r of records) {
    const v = r.attributes[key];
    if (typeof v === "string") values.add(v);
  }
  return [...values].sort();
}

export function distinctService(records: LogRecord[]): string[] {
  const values = new Set<string>();
  for (const r of records) {
    const v = r.resource["service.name"];
    if (typeof v === "string") values.add(v);
  }
  return [...values].sort();
}
