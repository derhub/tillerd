//! `Storage` aggregate and the cross-aggregate session-creation coordinator.

use std::sync::Arc;

use crate::entities::{NewSession, Session};
use crate::error::{OrchestratorError, Result};
use crate::infra::fs::FsBackend;
use crate::infra::memory::MemoryBackend;
use crate::infra::sqlite::SqliteBackend;
use crate::store::backend::Backend;
use crate::store::{
    Commands, LaunchTemplates, Notifications, Projects, Sessions, Settings, Surfaces, Workspaces,
};

/// All per-entity stores, constructed and owned at the composition root. Leaf consumers
/// receive only the concrete stores they use, never this whole aggregate.
pub struct Storage {
    pub workspaces: Workspaces,
    pub projects: Projects,
    pub sessions: Sessions,
    pub surfaces: Surfaces,
    pub commands: Commands,
    pub settings: Settings,
    pub notifications: Notifications,
    pub launch_templates: LaunchTemplates,
    /// Operational backend handle for the `schema_version` meta fn (not an entity store).
    operational: Backend,
}

impl Storage {
    /// Build every per-entity store over the given domain and operational backends.
    pub fn new(domain: Backend, operational: Backend) -> Self {
        Self {
            workspaces: Workspaces::new(domain.clone()),
            projects: Projects::new(domain.clone()),
            sessions: Sessions::new(domain.clone()),
            surfaces: Surfaces::new(domain),
            commands: Commands::new(operational.clone()),
            settings: Settings::new(operational.clone()),
            notifications: Notifications::new(operational.clone()),
            launch_templates: LaunchTemplates::new(operational.clone()),
            operational,
        }
    }

    /// Production storage: the fs domain tree plus the sqlite operational tables.
    pub fn open(fs: FsBackend, sqlite: SqliteBackend) -> Self {
        Self::new(Backend::Fs(Arc::new(fs)), Backend::Sqlite(Arc::new(sqlite)))
    }

    /// In-memory storage for tests: one `MemoryBackend` serving both planes.
    pub fn in_memory(mem: MemoryBackend) -> Self {
        let backend = Backend::Memory(Arc::new(mem));
        Self::new(backend.clone(), backend)
    }

    /// Operational schema version (meta/migration fn, not an entity store).
    pub async fn schema_version(&self) -> Result<u32> {
        self.operational.schema_version().await
    }
}

/// Resolve a draft's launch template into a concrete spec, then materialize the session.
///
/// Spans two aggregates (`LaunchTemplates` -> `Sessions`). By DDD layering this is application
/// work, not repository work: it is not a `Sessions` method (which must not depend on
/// `LaunchTemplates`) and not duplicated in the hosts. The composition root owns it.
pub async fn create_session(
    draft: NewSession,
    launch_templates: &LaunchTemplates,
    sessions: &Sessions,
) -> Result<Session> {
    let spec = match draft.template_id {
        Some(ref tid) => {
            let tmpl = launch_templates.get(tid.clone()).await?.ok_or_else(|| {
                OrchestratorError::LaunchTemplateNotFound(tid.as_str().to_string())
            })?;
            let instantiated = crate::launch::spec::instantiate_for_session(&tmpl.spec_json)?;
            Some((tmpl.spec_version, instantiated))
        }
        None => None,
    };
    sessions.create(draft, spec).await
}
