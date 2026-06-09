import * as fs from "node:fs";
import type { FileSource } from "@tillerd/sdk";

export class BunFileSource implements FileSource {
  async size(path: string): Promise<number | null> {
    try {
      return fs.statSync(path).size;
    } catch {
      return null;
    }
  }

  async read(path: string, offset: number, length: number): Promise<Uint8Array> {
    const fd = fs.openSync(path, "r");
    try {
      const buf = Buffer.alloc(length);
      const bytesRead = fs.readSync(fd, buf, 0, length, offset);
      return new Uint8Array(buf.buffer, buf.byteOffset, bytesRead);
    } finally {
      fs.closeSync(fd);
    }
  }
}
