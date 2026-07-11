import React from "react";

import { setWebviewZoom } from "~/lib/tauriEvents";

import { useNumberGlobalSetting } from "./context";
import { clampUiZoom, DEFAULT_UI_ZOOM, UI_ZOOM_KEY } from "./keys";

// Applies the persisted UI zoom level live to THIS window's webview (ui-settings-editor
// spec, "Zoom applies live"). Every window (main, detached, project, workspace) mounts
// this once at its root: on mount it applies the restored value (covers relaunch), and
// on every setting change -- local or a sibling window's write arriving over the
// cross-window settings sync -- it re-applies to this webview. No respawn, matching the
// live-apply pattern used for the terminal scheme (useLiveTerminalTheme).
export function useUiZoom(): { zoom: number; setZoom: (value: number) => void; reset: () => void } {
  const { value: zoom, setValue } = useNumberGlobalSetting(UI_ZOOM_KEY, DEFAULT_UI_ZOOM);

  React.useEffect(() => {
    void setWebviewZoom(zoom);
  }, [zoom]);

  const setZoom = React.useCallback((next: number) => setValue(clampUiZoom(next)), [setValue]);
  const reset = React.useCallback(() => setZoom(DEFAULT_UI_ZOOM), [setZoom]);

  return { zoom, setZoom, reset };
}
