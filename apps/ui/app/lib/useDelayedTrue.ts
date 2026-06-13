import { useEffect, useState } from "react";

/**
 * Returns `true` only once `active` has stayed true for `delayMs`, and resets to
 * `false` whenever `active` goes false. Defers a skeleton so content that resolves
 * within the grace window never flashes one.
 */
export function useDelayedTrue(active: boolean, delayMs: number): boolean {
  const [elapsed, setElapsed] = useState(false);
  useEffect(() => {
    if (!active) {
      setElapsed(false);
      return;
    }
    const id = setTimeout(() => setElapsed(true), delayMs);
    return () => clearTimeout(id);
  }, [active, delayMs]);
  return elapsed;
}
