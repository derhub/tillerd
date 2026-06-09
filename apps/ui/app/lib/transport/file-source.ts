import type { FileSource } from "@tillerd/sdk";
import type { TauriCore } from "./tauri";

export const FILE_SIZE = "file_size";
export const FILE_READ = "file_read";

/**
 * Native (web-view) `FileSource`: transcript reads over Rust `file_size`/`file_read` commands.
 * Off the hot byte path (hook-frequency, delta-sized), so per-call `invoke` is acceptable.
 */
export class TauriFileSource implements FileSource {
  constructor(private readonly core: TauriCore) {}

  /** Byte length, or `null` when the file is absent — distinct from an empty file (0). */
  size(path: string): Promise<number | null> {
    return this.core.invoke<number | null>(FILE_SIZE, { path });
  }

  async read(path: string, offset: number, length: number): Promise<Uint8Array> {
    const bytes = await this.core.invoke<unknown>(FILE_READ, { path, offset, length });
    return toBytes(bytes) ?? new Uint8Array(0);
  }
}

function toBytes(data: unknown): Uint8Array | null {
  if (data instanceof Uint8Array) return data;
  if (data instanceof ArrayBuffer) return new Uint8Array(data);
  if (Array.isArray(data)) return new Uint8Array(data as number[]);
  return null;
}
