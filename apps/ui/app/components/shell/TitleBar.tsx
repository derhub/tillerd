import { WindowControls } from "tauri-controls";

import { isDesktopHost } from "~/lib/transport/core";

// The custom title bar for the decorations-less desktop window: a drag region plus
// the OS window controls. tauri-controls drives the window through the Tauri 2 core
// (`__TAURI_INTERNALS__`), so the controls render only on the desktop host; the
// browser build shows a bare draggable bar. The toggle toolbar (titlebar-surface
// commands) mounts here once the command wiring lands.
export function TitleBar() {
  const desktop = isDesktopHost();
  return (
    <div
      data-tauri-drag-region
      className="flex h-9 shrink-0 items-center gap-1 border-b border-border/40 bg-background select-none"
    >
      {/* toolbar placeholder -- titlebar-surface command buttons mount here */}
      <div data-tauri-drag-region className="flex-1" />
      {desktop && <WindowControls platform="macos" />}
    </div>
  );
}
