import { AtError } from "@tillerd/sdk";

export class SendQueue {
  private queue: string[] = [];
  private ready = false;

  constructor(private readonly capacity: number) {}

  enqueue(text: string): void {
    if (this.queue.length >= this.capacity) {
      throw new AtError("QueueFull", `Send queue full (capacity ${this.capacity})`);
    }
    this.queue.push(text);
  }

  setReady(ready: boolean): string[] {
    this.ready = ready;
    if (ready) {
      const drained = this.queue.splice(0);
      return drained;
    }
    return [];
  }

  isReady(): boolean {
    return this.ready;
  }

  size(): number {
    return this.queue.length;
  }
}
