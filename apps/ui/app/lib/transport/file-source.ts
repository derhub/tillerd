import type { FileSource } from "@tillerd/sdk";

import type { TauriCore } from "./tauri";

import { toBytes } from "./bytes";

export const FILE_SIZE = "file_size";
export const FILE_READ = "file_read";

// Off the hot byte path (hook-frequency, delta-sized); per-call invoke is acceptable.
export class TauriFileSource implements FileSource {
  constructor(private readonly core: TauriCore) {}

  // null = file absent; distinct from an empty file (0).
  size(path: string): Promise<number | null> {
    return this.core.invoke<number | null>(FILE_SIZE, { path });
  }

  async read(path: string, offset: number, length: number): Promise<Uint8Array> {
    const bytes = await this.core.invoke<unknown>(FILE_READ, { path, offset, length });
    return toBytes(bytes) ?? new Uint8Array(0);
  }
}
