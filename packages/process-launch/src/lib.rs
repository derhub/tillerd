//! Run-others launcher for managed tool backends.
//!
//! Adopt a live, exact-version-matching instance when one is running; otherwise
//! spawn one and wait until its control socket is reachable, overwriting a stale
//! manifest that names a dead instance. Decide restarts by comparing only the
//! spawn-affecting fields, and restart an exited child with a capped exponential
//! backoff.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod adopt;
pub mod backoff;
pub mod diffing;
pub mod error;
pub mod manifest;
pub mod probes;
pub mod spawn;

pub use adopt::{AdoptMiss, Adoption};
pub use backoff::BackoffPolicy;
pub use diffing::{spawn_fields_differ, SpawnSpec};
pub use error::LaunchError;
pub use manifest::{athing_dir, ManifestData};
pub use probes::{OsProbes, Probes};
pub use spawn::{spawn_and_wait, SpawnTiming};

use std::path::Path;

/// A successfully launched backend: either an adopted live instance or a freshly
/// spawned one. In both cases the named pid serves the requested version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Launched {
    /// Connected to an already-running instance.
    Adopted {
        /// Pid of the adopted instance.
        pid: u32,
    },
    /// Spawned a new instance.
    Spawned {
        /// Pid of the spawned instance.
        pid: u32,
    },
}

impl Launched {
    /// The pid of the launched instance, regardless of how it was obtained.
    pub fn pid(self) -> u32 {
        match self {
            Launched::Adopted { pid } | Launched::Spawned { pid } => pid,
        }
    }
}

/// Adopt a live matching-version instance under `dir`, or spawn one and wait
/// until it is reachable.
///
/// Adoption requires an exact `version` match on the manifest (R3); any other
/// state — no manifest, a dead pid, a version mismatch, or an unresponsive
/// socket — falls through to spawn, which overwrites the stale manifest.
pub fn adopt_or_spawn(
    dir: &Path,
    version: &str,
    timing: &SpawnTiming,
    probes: &impl Probes,
) -> Result<Launched, LaunchError> {
    match adopt::evaluate(dir, version, probes) {
        Adoption::Adopted { pid } => Ok(Launched::Adopted { pid }),
        Adoption::Spawn(_) => {
            let pid = spawn::spawn_and_wait(dir, version, timing, probes)?;
            Ok(Launched::Spawned { pid })
        }
    }
}
