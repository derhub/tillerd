const CAPACITY = 64 * 1024; // 64 KB

export class ReplayBuffer {
  private chunks: Uint8Array[] = [];
  private totalBytes = 0;

  push(chunk: Uint8Array): void {
    this.chunks.push(chunk);
    this.totalBytes += chunk.length;
    while (this.totalBytes > CAPACITY) {
      const dropped = this.chunks.shift()!;
      this.totalBytes -= dropped.length;
    }
  }

  snapshot(): Uint8Array[] {
    return [...this.chunks];
  }
}
