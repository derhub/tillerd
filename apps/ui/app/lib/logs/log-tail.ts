import type { LogSource } from "../transport/log-source";
import { type LogRecord, parseRecord } from "./log-record";

export interface LogTailOptions {
  /** Max records kept in the live window before the oldest are trimmed. */
  windowSize?: number;
  /** Bytes read from the tail of each file when first seen. */
  backfillBytes?: number;
  /** Bytes read per {@link LogTail.loadOlder} step. */
  olderChunkBytes?: number;
}

const DEFAULTS = { windowSize: 2000, backfillBytes: 256 * 1024, olderChunkBytes: 64 * 1024 };

interface Cursor {
  /** Confirmed line boundary: lowest byte of the complete lines parsed so far. */
  start: number;
  /** Tail read position. */
  end: number;
  /** Partial trailing line (bytes after the last newline within the read range). */
  buf: string;
  decoder: TextDecoder;
}

const encoder = new TextEncoder();

/**
 * Tails a set of structured log files through a {@link LogSource}: polls each
 * file's size, reads the appended bytes, parses complete JSON lines (buffering a
 * partial trailing line until its newline arrives), and keeps a single window
 * merged across files by timestamp. {@link loadOlder} reads earlier ranges on
 * demand. The polling cadence lives in the caller, not here.
 */
export class LogTail {
  private readonly cursors = new Map<string, Cursor>();
  private records: LogRecord[] = [];
  private trimEnabled = true;
  private readonly windowSize: number;
  private readonly backfillBytes: number;
  private readonly olderChunkBytes: number;

  constructor(
    private readonly source: LogSource,
    opts: LogTailOptions = {},
  ) {
    this.windowSize = opts.windowSize ?? DEFAULTS.windowSize;
    this.backfillBytes = opts.backfillBytes ?? DEFAULTS.backfillBytes;
    this.olderChunkBytes = opts.olderChunkBytes ?? DEFAULTS.olderChunkBytes;
  }

  /** The current merged window. */
  current(): LogRecord[] {
    return this.records;
  }

  /** One poll cycle: pick up new bytes from every file. Returns the merged window. */
  async refresh(): Promise<LogRecord[]> {
    const files = await this.source.list();
    let added = false;
    for (const file of files) {
      const cursor = this.cursors.get(file.path);
      if (!cursor) {
        added = (await this.backfill(file.path, file.size)) || added;
      } else if (file.size > cursor.end) {
        added = (await this.readTail(file.path, cursor, file.size)) || added;
      }
    }
    if (added) this.commit();
    return this.records;
  }

  /** Read an earlier range of `path`, prepend its records. Returns the merged window. */
  async loadOlder(path: string): Promise<LogRecord[]> {
    const cursor = this.cursors.get(path);
    if (!cursor || cursor.start === 0) return this.records;
    this.trimEnabled = false;

    const top = cursor.start;
    const goal = Math.max(0, top - this.olderChunkBytes);
    const readFrom = goal > 0 ? goal - 1 : 0; // read one byte earlier to land on a boundary
    let text = new TextDecoder().decode(await this.source.read(path, readFrom, top - readFrom));

    if (readFrom > 0) {
      const nl = text.indexOf("\n");
      if (nl === -1) return this.records; // chunk holds no boundary; skip this step
      cursor.start = readFrom + encoder.encode(text.slice(0, nl + 1)).length;
      text = text.slice(nl + 1);
    } else {
      cursor.start = 0;
    }

    const older = this.parseComplete(trimTrailingEmpty(text.split("\n")));
    if (older.length) this.records = sortByTime(this.records.concat(older));
    return this.records;
  }

  /** Load an earlier range from every tracked file. Returns the merged window. */
  async loadOlderAll(): Promise<LogRecord[]> {
    for (const path of this.cursors.keys()) {
      await this.loadOlder(path);
    }
    return this.records;
  }

  private async backfill(path: string, size: number): Promise<boolean> {
    const decoder = new TextDecoder();
    if (size === 0) {
      this.cursors.set(path, { start: 0, end: 0, buf: "", decoder });
      return false;
    }
    const goal = Math.max(0, size - this.backfillBytes);
    const readFrom = goal > 0 ? goal - 1 : 0; // read one byte earlier to land on a boundary

    let text = decoder.decode(await this.source.read(path, readFrom, size - readFrom), {
      stream: true,
    });
    let boundary = readFrom;
    if (readFrom > 0) {
      const nl = text.indexOf("\n");
      if (nl === -1) {
        // backfill window holds no line boundary; hold it as a partial tail
        this.cursors.set(path, { start: size, end: size, buf: text, decoder });
        return false;
      }
      boundary = readFrom + encoder.encode(text.slice(0, nl + 1)).length;
      text = text.slice(nl + 1);
    }

    const lines = text.split("\n");
    const buf = lines.pop() ?? "";
    this.cursors.set(path, { start: boundary, end: size, buf, decoder });
    return this.push(this.parseComplete(lines));
  }

  private async readTail(path: string, cursor: Cursor, to: number): Promise<boolean> {
    const text = cursor.decoder.decode(await this.source.read(path, cursor.end, to - cursor.end), {
      stream: true,
    });
    cursor.end = to;
    const lines = (cursor.buf + text).split("\n");
    cursor.buf = lines.pop() ?? "";
    return this.push(this.parseComplete(lines));
  }

  private parseComplete(lines: string[]): LogRecord[] {
    const out: LogRecord[] = [];
    for (const line of lines) {
      const record = parseRecord(line);
      if (record) out.push(record);
    }
    return out;
  }

  private push(records: LogRecord[]): boolean {
    if (!records.length) return false;
    this.records.push(...records);
    return true;
  }

  private commit(): void {
    this.records = sortByTime(this.records);
    if (this.trimEnabled && this.records.length > this.windowSize) {
      this.records = this.records.slice(this.records.length - this.windowSize);
    }
  }
}

function sortByTime(records: LogRecord[]): LogRecord[] {
  return [...records].sort((a, b) =>
    a.timestamp < b.timestamp ? -1 : a.timestamp > b.timestamp ? 1 : 0,
  );
}

function trimTrailingEmpty(lines: string[]): string[] {
  if (lines.length && lines[lines.length - 1] === "") lines.pop();
  return lines;
}
