import React from "react";

import { subscribe } from "~/lib/subscribe";
import { armReattachOnClose } from "~/lib/windows";

/**
 * Arm a child window so any close path (native button or in-app re-attach) first emits its re-attach
 * event, then destroys the window -- the parent clears its detached indicator on that event. No-op
 * until `windowId` is set (i.e. this is actually a child window of that kind). `emit` is read through
 * a ref so re-arming depends only on the id, matching the original effect's dependency.
 */
export function useArmReattachOnClose(
  windowId: string | undefined,
  emit: (id: string) => Promise<void>,
): void {
  const emitRef = React.useRef(emit);
  emitRef.current = emit;
  React.useEffect(() => {
    if (!windowId) return;
    return subscribe(armReattachOnClose(() => emitRef.current(windowId)));
  }, [windowId]);
}
