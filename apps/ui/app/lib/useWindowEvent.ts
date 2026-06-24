import React from "react";

export function useWindowEvent(event: string, handler: () => void): void {
  const ref = React.useRef(handler);
  ref.current = handler;
  React.useEffect(() => {
    const fn = () => ref.current();
    window.addEventListener(event, fn);
    return () => window.removeEventListener(event, fn);
  }, [event]);
}
