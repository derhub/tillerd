//! Surface runtime: the `SurfaceRuntime` port, its daemon-socket adapter, the
//! `SurfaceEventSink` output port, and an in-memory fake for tests.
//!
//! The runtime owns the PTY proxies and the daemon transport; it does **no**
//! persistence. Output and status flow out through the [`SurfaceEventSink`]; the
//! app layer records status and reconciles desired rows against [`SurfaceRuntime::list`].

mod daemon;
mod fake;
mod transport;

pub use daemon::{DaemonRuntime, ResolvedCommand};
pub use fake::{FakeRuntime, RuntimeCall};
pub use transport::{default_daemon_socket, DaemonConnection, TransportError};

use std::future::Future;
use std::pin::Pin;

use crate::entities::{SurfaceId, SurfaceKind};
use crate::shared::Result;

/// A surface's output stream destination. The runtime pushes PTY bytes, status
/// transitions, and lifecycle events here; an implementor bridges them to a
/// renderer (tauri `ipc::Channel`) or another transport. Sync, so it stays
/// object-safe and adds no per-frame async overhead. Keystroke input never flows
/// through here — the sink carries daemon -> renderer output only.
pub trait SurfaceEventSink: Send + Sync + 'static {
    fn on_bytes(&self, surface: &SurfaceId, bytes: &[u8]);
    fn on_status(&self, surface: &SurfaceId, status: &str);
    fn on_exit(&self, surface: &SurfaceId, qualifier: &str);
    /// A non-recoverable surface-level error after open.
    fn on_error(&self, _surface: &SurfaceId, _reason: &str) {}
}

/// A fully resolved launch command: a concrete executable, its arguments, and
/// extra environment. `None` at a spawn site means the login shell.
pub type SpawnCommand = ResolvedCommand;

/// Geometry a surface is spawned with; the renderer resizes on attach.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Geometry {
    pub cols: u16,
    pub rows: u16,
}

/// Everything the runtime needs to bring a surface's PTY to life.
#[derive(Debug, Clone)]
pub struct SpawnRequest {
    pub surface: SurfaceId,
    pub kind: SurfaceKind,
    pub command: Option<SpawnCommand>,
    pub token: String,
    pub geometry: Geometry,
    pub cwd: String,
}

type BoxFut<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'a>>;

/// The surface runtime port: drives PTYs in the daemon and streams their output to
/// a [`SurfaceEventSink`]. Object-safe (`Arc<dyn SurfaceRuntime>` in `Ctx`); methods
/// return boxed futures so the trait stays `dyn`-compatible without `async-trait`.
/// The port does no persistence — the app layer owns intent and reconciliation (D9).
pub trait SurfaceRuntime: Send + Sync + 'static {
    /// Spawn a surface's PTY and start proxying its output to the sink.
    fn spawn<'a>(&'a self, request: SpawnRequest) -> BoxFut<'a, ()>;

    /// Stop the PTY (terminate, suppress auto-resume) and drop the proxy; the
    /// surface record is kept (resumable later).
    fn stop<'a>(&'a self, surface: &'a SurfaceId) -> BoxFut<'a, ()>;

    /// Kill the PTY and drop the proxy; the surface record is discarded by the caller.
    fn close<'a>(&'a self, surface: &'a SurfaceId) -> BoxFut<'a, ()>;

    /// Enumerate the ids of every PTY currently live in the daemon (for the boot
    /// reconciler).
    fn list<'a>(&'a self) -> BoxFut<'a, Vec<SurfaceId>>;

    /// Send raw input bytes to a surface's PTY stdin. Off the bus, never logged.
    fn input<'a>(&'a self, surface: &'a SurfaceId, bytes: &'a [u8]) -> BoxFut<'a, ()>;

    /// Resize a surface's PTY.
    fn resize<'a>(&'a self, surface: &'a SurfaceId, cols: u16, rows: u16) -> BoxFut<'a, ()>;

    /// Connect the proxy stream to an already-running daemon PTY (lazy, per surface).
    fn attach<'a>(&'a self, surface: &'a SurfaceId) -> BoxFut<'a, ()>;

    /// Drop the proxy stream; the PTY keeps running in the daemon.
    fn detach<'a>(&'a self, surface: &'a SurfaceId) -> BoxFut<'a, ()>;
}
