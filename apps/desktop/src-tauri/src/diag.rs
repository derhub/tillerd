use serde_json::Value;

/// Diagnostic sink backing the renderer `Logger` forward. Writes to stderr (captured in the app
/// log); never fails into the caller.
#[tauri::command]
pub fn log_forward(level: String, msg: String, extra: Option<Value>) {
    match extra {
        Some(extra) => eprintln!("[renderer:{level}] {msg} {extra}"),
        None => eprintln!("[renderer:{level}] {msg}"),
    }
}
