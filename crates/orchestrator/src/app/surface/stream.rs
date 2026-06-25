//! The surface output pump: pulls raw frames from the runtime and fans them out
//! to all registered sinks as borrowed `SurfaceEvent`s -- zero extra copy on the
//! dispatch path.

use std::sync::Arc;

use crate::events::surface::{SurfaceEvent, SurfaceSink};
use crate::infra::daemon_pty_api::{Output, Runtime};
use crate::shared::bus::Registry;

/// Bridges the raw pull source (infra `Runtime`) to the key-scoped sink layer.
///
/// Construct once at boot, then `tokio::spawn(stream.run())`. The future exits
/// when the runtime channel closes (all proxy tasks dropped). Each frame is
/// delivered to the `registry` sinks subscribed to that frame's surface, exactly
/// once; delivery stays borrowed and zero-copy.
pub struct SurfaceStream {
    pub runtime: Arc<Runtime>,
    pub registry: Arc<Registry<dyn SurfaceSink>>,
}

impl SurfaceStream {
    pub async fn run(self) {
        while let Some(frame) = self.runtime.recv().await {
            let event = match &frame.output {
                Output::Bytes(b) => SurfaceEvent::Bytes(b),
                Output::Status(s) => SurfaceEvent::Status(s),
                Output::Exit(q) => SurfaceEvent::Exit(q),
                Output::Error(r) => SurfaceEvent::Error(r),
            };
            self.registry
                .dispatch(&frame.surface, |s| s.emit(&frame.surface, &event));
        }
    }
}
