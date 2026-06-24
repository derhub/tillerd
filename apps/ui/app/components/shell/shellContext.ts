import React from "react";

import type { PanelLeaf } from "~/lib/panelTree";

export interface DetachedPanelsValue {
  detached: Set<string>;
  detach: (leaf: PanelLeaf) => void;
  reattach: (placement: string) => void;
}

export const DetachedPanelsContext = React.createContext<DetachedPanelsValue>({
  detached: new Set(),
  detach: () => {},
  reattach: () => {},
});
