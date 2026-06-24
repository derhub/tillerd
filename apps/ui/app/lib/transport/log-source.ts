import type { TauriCore } from "./tauri";

import { withDesktopCore } from "./core";
import { TauriFileSource } from "./file-source";

export interface LogFileInfo {
  name: string;
  path: string;
  size: number;
}

export interface LogSource {
  list(): Promise<LogFileInfo[]>;
  size(path: string): Promise<number | null>;
  read(path: string, offset: number, length: number): Promise<Uint8Array>;
}

export const LIST_LOG_FILES = "list_log_files";

export class TauriLogSource implements LogSource {
  private readonly files: TauriFileSource;

  constructor(private readonly core: TauriCore) {
    this.files = new TauriFileSource(core);
  }

  list(): Promise<LogFileInfo[]> {
    return this.core.invoke<LogFileInfo[]>(LIST_LOG_FILES);
  }

  size(path: string): Promise<number | null> {
    return this.files.size(path);
  }

  read(path: string, offset: number, length: number): Promise<Uint8Array> {
    return this.files.read(path, offset, length);
  }
}

export function loadLogSource(): Promise<LogSource | null> {
  return withDesktopCore((core) => new TauriLogSource(core));
}
