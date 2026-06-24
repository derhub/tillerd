//! Surface runtime: a concrete `DaemonPtyApi` over the daemon socket, an
//! in-memory `FakeRuntime` for tests, and the `Runtime` enum that provides
//! static dispatch between them without boxing.
//!
//! The `SurfaceRuntime` trait and its `Arc<dyn>` / boxed-future machinery are
//! gone. The runtime owns the PTY proxies and the daemon transport; it does
//! **no** persistence. Output is pulled via [`Runtime::recv`]; the app layer
//! records status and reconciles desired rows against [`Runtime::list`].

mod daemon;
mod fake;
mod transport;

pub use daemon::{DaemonPtyApi, ResolvedCommand};
pub use fake::FakeRuntime;
#[cfg(test)]
pub use fake::RuntimeCall;

use std::sync::Arc;

use crate::entities::SurfaceId;
use crate::shared::Result;

/// One decoded output frame from a surface PTY proxy. All payloads are owned
/// so they can cross the mpsc channel boundary without a lifetime.
pub enum Output {
    Bytes(Vec<u8>),
    Status(String),
    Exit(String),
    Error(String),
}

/// A decoded frame addressed to a specific surface.
pub struct SurfaceOutput {
    /// Primitive surface id (no entity newtype here -- infra stays plain).
    pub surface: String,
    pub output: Output,
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
/// Kind is absent: the daemon is kind-agnostic; capability checks live in app.
#[derive(Debug, Clone)]
pub struct SpawnRequest {
    pub surface: SurfaceId,
    pub command: Option<SpawnCommand>,
    pub token: String,
    pub geometry: Geometry,
    pub cwd: String,
}

/// The surface runtime the composition root holds. Static dispatch over the two
/// concrete impls -- no `Arc<dyn>`, no boxed futures.
///
/// `Fake` wraps `Arc<FakeRuntime>` so tests can retain an `Arc<FakeRuntime>`
/// handle for assertions (calls, is_running, seed_running) while `Ctx` wraps
/// `Runtime` in its own `Arc` for clone semantics.
pub enum Runtime {
    Daemon(DaemonPtyApi),
    Fake(Arc<FakeRuntime>),
}

impl Runtime {
    pub async fn spawn(&self, request: SpawnRequest) -> Result<()> {
        match self {
            Self::Daemon(r) => r.spawn(request).await,
            Self::Fake(r) => r.spawn(request).await,
        }
    }

    pub async fn stop(&self, surface: &SurfaceId) -> Result<()> {
        match self {
            Self::Daemon(r) => r.stop(surface).await,
            Self::Fake(r) => r.stop(surface).await,
        }
    }

    pub async fn close(&self, surface: &SurfaceId) -> Result<()> {
        match self {
            Self::Daemon(r) => r.close(surface).await,
            Self::Fake(r) => r.close(surface).await,
        }
    }

    pub async fn list(&self) -> Result<Vec<SurfaceId>> {
        match self {
            Self::Daemon(r) => r.list().await,
            Self::Fake(r) => r.list().await,
        }
    }

    pub async fn input(&self, surface: &SurfaceId, bytes: &[u8]) -> Result<()> {
        match self {
            Self::Daemon(r) => r.input(surface, bytes).await,
            Self::Fake(r) => r.input(surface, bytes).await,
        }
    }

    pub async fn resize(&self, surface: &SurfaceId, cols: u16, rows: u16) -> Result<()> {
        match self {
            Self::Daemon(r) => r.resize(surface, cols, rows).await,
            Self::Fake(r) => r.resize(surface, cols, rows).await,
        }
    }

    pub async fn attach(&self, surface: &SurfaceId) -> Result<()> {
        match self {
            Self::Daemon(r) => r.attach(surface).await,
            Self::Fake(r) => r.attach(surface).await,
        }
    }

    pub async fn detach(&self, surface: &SurfaceId) -> Result<()> {
        match self {
            Self::Daemon(r) => r.detach(surface).await,
            Self::Fake(r) => r.detach(surface).await,
        }
    }

    /// Pull the next decoded output frame from any surface. Returns `None` when
    /// the internal channel is closed (daemon variant: channel dropped; fake
    /// variant: nothing enqueued).
    pub async fn recv(&self) -> Option<SurfaceOutput> {
        match self {
            Self::Daemon(r) => r.recv().await,
            Self::Fake(r) => r.recv().await,
        }
    }
}
