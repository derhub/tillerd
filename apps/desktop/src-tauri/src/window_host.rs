//! Child-window primitives for panel detach / multi-window (roadmap 0.0.11). All windows share
//! this one backend process and its `SurfaceState`, so a child window is just another webview of
//! the same app; opening one spawns no backend. `window_open` loads the renderer at the root with
//! an intent query (`?w=detached&...` / `?w=project&...`) the shell reads — a non-root deep route
//! has no SPA fallback under the custom scheme. A child can close itself via the core window API;
//! `window_close` lets the parent re-attach by closing a child by label (its close handler emits
//! the re-attach event).

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

#[tauri::command]
pub async fn window_open<R: tauri::Runtime>(
    app: AppHandle<R>,
    label: String,
    query: String,
) -> Result<(), String> {
    if let Some(existing) = app.get_webview_window(&label) {
        return existing.set_focus().map_err(|e| e.to_string());
    }
    WebviewWindowBuilder::new(&app, &label, WebviewUrl::App(query.into()))
        .title("tillerd")
        .inner_size(820.0, 640.0)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn window_focus<R: tauri::Runtime>(app: AppHandle<R>, label: String) -> Result<(), String> {
    let window = app
        .get_webview_window(&label)
        .ok_or_else(|| format!("no window {label}"))?;
    window.set_focus().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn window_close<R: tauri::Runtime>(app: AppHandle<R>, label: String) -> Result<(), String> {
    // close() (not destroy()) so the child's onCloseRequested handler runs — that handler emits the
    // re-attach event the parent listens for. A missing window is a no-op (already re-attached).
    if let Some(window) = app.get_webview_window(&label) {
        window.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}
