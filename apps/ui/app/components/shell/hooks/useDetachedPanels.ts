import React from "react";

import type { PanelLeaf } from "~/lib/panelTree";

import { subscribe } from "~/lib/subscribe";
import {
  closeWindow,
  detachedLabel,
  detachedQuery,
  focusSelf,
  onReattachPanel,
  openWindow,
} from "~/lib/windows";

export interface DetachedPanels {
  detached: Set<string>;
  detachedRef: React.RefObject<Set<string>>;
  detach: (leaf: PanelLeaf) => void;
  reattach: (placement: string) => void;
}

// The detached set is renderer-runtime only -- not written to layout_json -- so a relaunch starts
// fully attached. Parent windows listen for child re-attach to clear the flag; child windows skip that.
export function useDetachedPanels(
  sessionId: string | null,
  isProjectWindow: boolean,
): DetachedPanels {
  const [detached, setDetached] = React.useState<Set<string>>(() => new Set());
  const detachedRef = React.useRef(detached);
  detachedRef.current = detached;

  const clear = React.useCallback((placement: string) => {
    setDetached((prev) => {
      if (!prev.has(placement)) return prev;
      const next = new Set(prev);
      next.delete(placement);
      return next;
    });
  }, []);

  const detach = React.useCallback(
    (leaf: PanelLeaf) => {
      if (leaf.content.type !== "terminal" || !sessionId) return;
      const placement = leaf.content.placement;
      void openWindow(detachedLabel(placement), detachedQuery(sessionId, placement));
      setDetached((prev) => new Set(prev).add(placement));
    },
    [sessionId],
  );

  // Close the child and clear immediately -- the child's re-attach event may not fire if the child
  // is closed before it arms its close handler.
  const reattach = React.useCallback(
    (placement: string) => {
      void closeWindow(detachedLabel(placement));
      clear(placement);
    },
    [clear],
  );

  React.useEffect(() => {
    if (isProjectWindow) return;
    return subscribe(
      onReattachPanel(({ placement }) => {
        clear(placement);
        void focusSelf();
      }),
    );
  }, [isProjectWindow, clear]);

  return { detached, detachedRef, detach, reattach };
}
