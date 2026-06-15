import { useEffect, useRef } from "react";

/**
 * Subscribe to a window event for the component's lifetime. The handler is read through a ref so an
 * inline closure does not re-bind the listener every render.
 */
export function useWindowEvent(event: string, handler: () => void): void {
  const ref = useRef(handler);
  ref.current = handler;
  useEffect(() => {
    const fn = () => ref.current();
    window.addEventListener(event, fn);
    return () => window.removeEventListener(event, fn);
  }, [event]);
}
