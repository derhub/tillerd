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
pub mod resolve_or_spawn;
pub mod surface_channel;
mod view;

pub mod close_surface;
pub mod detach_surface;
pub mod find_surface_by_placement;
pub mod get_surface_by_id;
pub mod list_resumable_surfaces;
pub mod list_surfaces_by_session;
pub mod reconcile_surfaces;
pub mod spawn_surface;
pub mod status_events;
pub mod stop_surface;

#[cfg(test)]
pub(crate) mod test_util;

pub use close_surface::CloseSurface;
pub use detach_surface::DetachSurface;
pub use find_surface_by_placement::FindSurfaceByPlacement;
pub use get_surface_by_id::GetSurfaceById;
pub use list_resumable_surfaces::ListResumableSurfaces;
pub use list_surfaces_by_session::ListSurfacesBySession;
pub use reconcile_surfaces::ReconcileSurfaces;
pub use resolve_or_spawn::ResolveOrSpawnSurface;
pub use spawn_surface::SpawnSurface;
pub use status_events::{
    update_status_and_emit, CloseSurfaceStatusChannel, OpenSurfaceStatusChannel,
};
pub use stop_surface::StopSurface;
pub use surface_channel::{
    CloseSurfaceChannel, OpenSurfaceChannel, SurfaceChannelStream, SurfaceClientMsg,
};
pub use view::SurfaceView;
