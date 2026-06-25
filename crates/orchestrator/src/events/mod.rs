//! Internal event transport contracts. Each sub-module pairs a borrowed-enum
//! event type with a sink trait; `app` re-exports the public surface for the
//! host. The module is `pub(crate)` -- the host imports from `app`, never here.

pub mod log;
pub mod notification;
pub mod surface;
