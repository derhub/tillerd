//! The host-facing surface output port. The runtime pushes PTY bytes, status
//! transitions, and lifecycle events here, addressed by primitive surface id; an
//! implementor bridges them to a renderer (tauri `ipc::Channel`) or another
//! transport. Sync, so it stays object-safe and adds no per-frame async overhead.
//! Keystroke input never flows through here -- this carries daemon -> renderer
//! output only.

/// A surface's output stream destination, addressed by primitive surface id.
pub trait SurfaceEvents: Send + Sync + 'static {
    fn on_bytes(&self, surface: &str, bytes: &[u8]);
    fn on_status(&self, surface: &str, status: &str);
    fn on_exit(&self, surface: &str, qualifier: &str);
    /// A non-recoverable surface-level error after open.
    fn on_error(&self, _surface: &str, _reason: &str) {}
}
