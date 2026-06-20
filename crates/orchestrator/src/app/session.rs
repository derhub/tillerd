//! Session use cases: cross-aggregate coordination over the per-entity stores.

use crate::entities::{NewSession, Session, SessionId};
use crate::error::{OrchestratorError, Result};
use crate::store::{LaunchTemplates, Sessions};

/// Brings a freshly created session's surfaces to life. The production implementation dispatches to
/// the surface runtime; tests record the calls. Mirrors `SurfaceLauncher`: RPITIT, no `async-trait`
/// and no `dyn`, keeping the app layer independent of the concrete surface runtime.
pub trait SessionActivator {
    fn activate(&self, session_id: &SessionId) -> impl std::future::Future<Output = Result<()>>;
}

/// Resolve a draft's launch template into a concrete spec, then materialize the session.
///
/// Spans two aggregates (`LaunchTemplates` -> `Sessions`). By DDD layering this is application
/// work, not repository work: it is not a `Sessions` method (which must not depend on
/// `LaunchTemplates`) and not duplicated in the hosts. The app layer owns it.
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

/// Open a session: create it, then best-effort activate its surfaces.
///
/// Activation runs through the [`SessionActivator`] port so this use case is host-agnostic. A
/// launch failure is non-fatal — it is logged and the created session is still returned, preserving
/// the desktop host's prior create-then-launch behavior.
pub async fn open_session(
    draft: NewSession,
    launch_templates: &LaunchTemplates,
    sessions: &Sessions,
    activator: &impl SessionActivator,
) -> Result<Session> {
    let session = create_session(draft, launch_templates, sessions).await?;
    if let Err(e) = activator.activate(&session.id).await {
        tracing::warn!(
            session_id = %session.id.as_str(),
            error = %e,
            "session activation failed (non-fatal)"
        );
    }
    Ok(session)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::infra::memory::MemoryBackend;
    use crate::store::Storage;

    struct FakeActivator {
        calls: Mutex<Vec<SessionId>>,
        fail: bool,
    }

    impl FakeActivator {
        fn new(fail: bool) -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                fail,
            }
        }
    }

    impl SessionActivator for FakeActivator {
        async fn activate(&self, session_id: &SessionId) -> Result<()> {
            self.calls.lock().unwrap().push(session_id.clone());
            if self.fail {
                Err(OrchestratorError::Persistence("boom".into()))
            } else {
                Ok(())
            }
        }
    }

    #[tokio::test]
    async fn open_session_persists_the_session_and_activates_it_once() {
        let storage = Storage::in_memory(MemoryBackend::new());
        let activator = FakeActivator::new(false);

        let session = open_session(
            NewSession::default(),
            &storage.launch_templates,
            &storage.sessions,
            &activator,
        )
        .await
        .unwrap();

        assert!(storage
            .sessions
            .get(session.id.clone())
            .await
            .unwrap()
            .is_some());
        let calls = activator.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].as_str(), session.id.as_str());
    }

    #[tokio::test]
    async fn open_session_returns_the_session_when_activation_fails() {
        let storage = Storage::in_memory(MemoryBackend::new());
        let activator = FakeActivator::new(true);

        let session = open_session(
            NewSession::default(),
            &storage.launch_templates,
            &storage.sessions,
            &activator,
        )
        .await
        .unwrap();

        assert!(storage.sessions.get(session.id).await.unwrap().is_some());
    }
}
