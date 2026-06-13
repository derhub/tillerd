/** A parsed structured log record, normalized to the OpenTelemetry field groups. */
export interface LogRecord {
  timestamp: string;
  level: string;
  body: string;
  attributes: Record<string, unknown>;
  resource: Record<string, unknown>;
  raw: string;
}

/** Keys that belong to the per-process resource, not to record attributes. */
const RESOURCE_KEYS = new Set([
  "service.name",
  "service.version",
  "service.instance.id",
  "host.name",
  "process.pid",
]);

/**
 * Parse one JSON log line (the `tracing-subscriber` JSON shape: top-level
 * `timestamp` / `level`, a `fields` object carrying `message`, and a `spans`
 * stack carrying span fields) into a {@link LogRecord}. Returns `null` for a
 * blank line or one that is not valid JSON, so a malformed line is skipped
 * rather than shown.
 */
export function parseRecord(line: string): LogRecord | null {
  const trimmed = line.trim();
  if (!trimmed) return null;
  let obj: unknown;
  try {
    obj = JSON.parse(trimmed);
  } catch {
    return null;
  }
  if (typeof obj !== "object" || obj === null) return null;
  const record = obj as Record<string, unknown>;

  const fields = asRecord(record.fields);
  const body = typeof fields.message === "string" ? fields.message : "";

  const attributes: Record<string, unknown> = {};
  const resource: Record<string, unknown> = {};
  const place = (key: string, value: unknown) => {
    if (key === "message" || key === "name") return;
    if (RESOURCE_KEYS.has(key)) resource[key] = value;
    else attributes[key] = value;
  };

  for (const [key, value] of Object.entries(fields)) place(key, value);
  for (const span of spanList(record)) {
    for (const [key, value] of Object.entries(span)) place(key, value);
  }

  return {
    timestamp: typeof record.timestamp === "string" ? record.timestamp : "",
    level: typeof record.level === "string" ? record.level : "",
    body,
    attributes,
    resource,
    raw: trimmed,
  };
}

function asRecord(value: unknown): Record<string, unknown> {
  return typeof value === "object" && value !== null ? (value as Record<string, unknown>) : {};
}

function spanList(record: Record<string, unknown>): Record<string, unknown>[] {
  if (Array.isArray(record.spans)) {
    return record.spans.filter(
      (s): s is Record<string, unknown> => typeof s === "object" && s !== null,
    );
  }
  if (typeof record.span === "object" && record.span !== null) {
    return [record.span as Record<string, unknown>];
  }
  return [];
}
