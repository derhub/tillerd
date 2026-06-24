import type { LogSource } from "../transport/log-source";

import { type LogRecord, parseRecord } from "./log-record";

export interface LogTailOptions {
  windowSize?: number;
  maxRecords?: number;
  backfillBytes?: number;
  olderChunkBytes?: number;
}

const DEFAULTS = {
  windowSize: 2000,
  maxRecords: 10_000,
  backfillBytes: 256 * 1024,
  olderChunkBytes: 64 * 1024,
};

interface Cursor {
  start: number;
  end: number;
  buf: string;
  decoder: TextDecoder;
}

const encoder = new TextEncoder();

export class LogTail {
  private readonly cursors = new Map<string, Cursor>();
  private records: LogRecord[] = [];
  private trimEnabled = true;
  private readonly windowSize: number;
  private readonly maxRecords: number;
  private readonly backfillBytes: number;
  private readonly olderChunkBytes: number;

  constructor(
    private readonly source: LogSource,
    opts: LogTailOptions = {},
  ) {
    this.windowSize = opts.windowSize ?? DEFAULTS.windowSize;
    this.maxRecords = opts.maxRecords ?? DEFAULTS.maxRecords;
    this.backfillBytes = opts.backfillBytes ?? DEFAULTS.backfillBytes;
    this.olderChunkBytes = opts.olderChunkBytes ?? DEFAULTS.olderChunkBytes;
  }

  current(): LogRecord[] {
    return this.records;
  }

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

  async loadOlder(path: string): Promise<LogRecord[]> {
    const cursor = this.cursors.get(path);
    if (!cursor || cursor.start === 0) return this.records;
    this.trimEnabled = false;

    const top = cursor.start;
    const goal = Math.max(0, top - this.olderChunkBytes);
    const readFrom = goal > 0 ? goal - 1 : 0; // read one extra byte to land on a line boundary
    let text = new TextDecoder().decode(await this.source.read(path, readFrom, top - readFrom));

    if (readFrom > 0) {
      const advanced = afterFirstLine(text, readFrom);
      if (!advanced) return this.records;
      cursor.start = advanced.boundary;
      text = advanced.rest;
    } else {
      cursor.start = 0;
    }

    const older = this.parseComplete(trimTrailingEmpty(text.split("\n")));
    if (older.length) {
      this.records = sortByTime(this.records.concat(older));
      if (this.records.length > this.maxRecords) {
        this.records = this.records.slice(this.records.length - this.maxRecords);
      }
    }
    return this.records;
  }

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
    const readFrom = goal > 0 ? goal - 1 : 0; // read one extra byte to land on a line boundary

    let text = decoder.decode(await this.source.read(path, readFrom, size - readFrom), {
      stream: true,
    });
    let boundary = readFrom;
    if (readFrom > 0) {
      const advanced = afterFirstLine(text, readFrom);
      if (!advanced) {
        this.cursors.set(path, { start: size, end: size, buf: text, decoder });
        return false;
      }
      boundary = advanced.boundary;
      text = advanced.rest;
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
    return lines.map((line) => parseRecord(line)).filter((r): r is LogRecord => r !== null);
  }

  private push(records: LogRecord[]): boolean {
    if (!records.length) return false;
    this.records.push(...records);
    return true;
  }

  private commit(): void {
    this.records = sortByTime(this.records);
    const cap = this.trimEnabled ? this.windowSize : this.maxRecords;
    if (this.records.length > cap) {
      this.records = this.records.slice(this.records.length - cap);
    }
  }
}

// Drops the first (partial) line of a chunk starting at byte `readFrom`.
// Returns the remaining complete-line text and the absolute byte offset where it starts.
// Null when the chunk holds no newline.
function afterFirstLine(text: string, readFrom: number): { rest: string; boundary: number } | null {
  const nl = text.indexOf("\n");
  if (nl === -1) return null;
  return {
    rest: text.slice(nl + 1),
    boundary: readFrom + encoder.encode(text.slice(0, nl + 1)).length,
  };
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
