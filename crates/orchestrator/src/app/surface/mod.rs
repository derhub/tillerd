//! Surface use cases. Two dispatch paths share one layer:
//!
//! - **Bus commands** (`SpawnSurface`/`StopSurface`/`CloseSurface`/`DetachSurface`/
//!   `ReconcileSurfaces`) and queries (`GetSurfaceById`/`FindSurfaceByPlacement`/
//!   `ListResumableSurfaces`/`ListSurfacesBySession`) go through the `Bus` -- persist
//!   and coordinate, telemetry on.
//! - **Off-bus I/O channel** (`send_surface_input`/`resize_surface`/`attach_surface`)
//!   are plain `app` functions the host calls directly. They skip the bus entirely:
//!   no command object, no span, no telemetry -- a keystroke must never reach a log.
//!   They still live in `app/` so the host never touches `infra`.
//!
//! Side effects follow D9: persist intent (a `pending` row) committed before the
//! spawn, run the spawn lock-free against the runtime port, then record the outcome
//! (`live`/`failed`). The DB is the source of truth for intent; `ReconcileSurfaces`
//! converges the runtime to match on boot -- without attaching any stream.

mod common;
pub(crate) mod stream;
mod view;

pub mod close_surface;
pub mod detach_surface;
pub mod find_surface_by_placement;
pub mod get_surface_by_id;
pub mod io_channel;
pub mod list_resumable_surfaces;
pub mod list_surfaces_by_session;
pub mod reconcile_surfaces;
pub mod spawn_surface;
pub mod stop_surface;
pub mod subscribe;

#[cfg(test)]
pub(crate) mod test_util;

pub use close_surface::CloseSurface;
pub use detach_surface::DetachSurface;
pub use find_surface_by_placement::FindSurfaceByPlacement;
pub use get_surface_by_id::GetSurfaceById;
pub use list_resumable_surfaces::ListResumableSurfaces;
pub use list_surfaces_by_session::ListSurfacesBySession;
pub use reconcile_surfaces::ReconcileSurfaces;
pub use spawn_surface::SpawnSurface;
pub use stop_surface::StopSurface;
pub use subscribe::{SubscribeSurface, UnsubscribeSurface};
pub use view::SurfaceView;

pub use common::{attach_surface, resize_surface, send_surface_input};

// The host's tauri transport implements `SurfaceSink` and registers a
// per-surface sink via `SubscribeSurface`. Both speak primitive surface ids,
// so the host never reaches the domain newtype or the infra layer.
pub use crate::events::surface::{SurfaceEvent, SurfaceSink};
pub use stream::SurfaceStream;
