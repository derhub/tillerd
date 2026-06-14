## Why

Users need to view multiple panels simultaneously on multi-monitor setups. Panel detach and project windows (picture-in-picture) complete the working app's core UX. The orchestrator's event sink already supports multiple concurrent subscribers (one per window); no backend connectivity changes required.

## What Changes

- **Panel detach button**: Panel header gets a detach icon that spawns a new child window holding that panel. Parent shows a greyed-out placeholder with a "Focus →" button to bring the child to front.
- **Project in new window**: Right-click on a project in the sidebar → "Open in new window"; parent sidebar shows a pending-detach indicator.
- **Re-attach flow**: Child window has a "Re-attach" action that returns the panel to its parent and auto-focuses the parent.
- **Parent close isolation**: Closing the parent window does not affect detached child windows; they remain independent.
- **Live-panel constraint**: Only panels with a live surface (terminal, eventually diff) support detach; empty panels do not.

## Capabilities

### New Capabilities

- `panel-detach`: Detach button in panel header → new child window; greyed placeholder in parent; "Focus →" button; child re-attach action.
- `project-in-new-window`: Right-click project sidebar entry → "Open in new window"; parent sidebar shows pending-detach badge.

### Modified Capabilities

- `window-lifecycle`: Child windows are independent; closing parent does not cascade. Tauri native API, no schema change to placement/surface binding.

## Impact

- **Frontend**: New UI (detach button, re-attach action, query param detection for child window context).
- **Backend**: `window_host.rs` command primitives (`window_open`, `window_focus`) already exist; no new Rust work.
- **State**: Window scope tracking via Tauri window-state plugin or localStorage for detached state (placement → window map).
- **Persistence**: Child window geometry persisted via Tauri window-state or manual localStorage; detach state (which placements are detached) stored or computed from active windows.
- **E2E**: Extend tauri-webdriver suite with panel detach → "Focus →" → re-attach flows on macOS and Linux CI.
