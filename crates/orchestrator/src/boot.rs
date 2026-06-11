use std::sync::Arc;

use crate::error::{OrchestratorError, Result};
use crate::persistence::Store;
use crate::supervision::{all_available, ServiceStatus, Supervise};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Booting,
    OpeningStore,
    Supervising,
    Ready,
    Failed { reason: String },
}

pub trait EventSink {
    fn emit(&self, event: &Status);
}

pub struct Orchestrator {
    status: Status,
    store: Arc<dyn Store>,
    services: Vec<ServiceStatus>,
}

impl Orchestrator {
    pub fn status(&self) -> &Status {
        &self.status
    }

    pub fn is_ready(&self) -> bool {
        self.status == Status::Ready
    }

    pub fn store(&self) -> &dyn Store {
        self.store.as_ref()
    }

    /// A shared handle to the durable store, for subsystems (e.g. the surface
    /// runtime) that outlive a single call.
    pub fn store_arc(&self) -> Arc<dyn Store> {
        Arc::clone(&self.store)
    }

    pub fn service_statuses(&self) -> &[ServiceStatus] {
        &self.services
    }
}

pub fn boot<F>(
    open_store: F,
    supervisor: &mut impl Supervise,
    sink: &impl EventSink,
) -> Result<Orchestrator>
where
    F: FnOnce() -> Result<Box<dyn Store>>,
{
    sink.emit(&Status::Booting);

    sink.emit(&Status::OpeningStore);
    let store: Arc<dyn Store> = Arc::from(fail_on(open_store(), sink)?);

    sink.emit(&Status::Supervising);
    let services = fail_on(supervisor.ensure_all(), sink)?;
    if !all_available(&services) {
        let unavailable = services
            .iter()
            .filter(|s| !s.is_available())
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(emit_failure(
            OrchestratorError::ServiceUnavailable {
                service: unavailable,
                reason: "service not available at boot".to_string(),
            },
            sink,
        ));
    }

    let orchestrator = Orchestrator {
        status: Status::Ready,
        store,
        services,
    };
    sink.emit(&Status::Ready);
    Ok(orchestrator)
}

fn emit_failure(error: OrchestratorError, sink: &impl EventSink) -> OrchestratorError {
    sink.emit(&Status::Failed {
        reason: error.to_string(),
    });
    error
}

fn fail_on<T>(step: Result<T>, sink: &impl EventSink) -> Result<T> {
    step.map_err(|e| emit_failure(e, sink))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::current_schema_version;
    use crate::persistence::memory::InMemoryStore;
    use crate::supervision::Liveness;
    use std::sync::Mutex;

    #[derive(Default)]
    struct RecordingSink {
        events: Mutex<Vec<Status>>,
    }

    impl RecordingSink {
        fn events(&self) -> Vec<Status> {
            self.events.lock().unwrap().clone()
        }
    }

    impl EventSink for RecordingSink {
        fn emit(&self, event: &Status) {
            self.events.lock().unwrap().push(event.clone());
        }
    }

    enum FakeSupervisor {
        AllAvailable,
        OneUnavailable,
        Errors,
    }

    fn status(name: &str, liveness: Liveness) -> ServiceStatus {
        ServiceStatus {
            name: name.to_string(),
            version: Some("1.0.0".to_string()),
            liveness,
            pid: Some(1),
            adopted: true,
        }
    }

    impl Supervise for FakeSupervisor {
        fn ensure_all(&mut self) -> Result<Vec<ServiceStatus>> {
            match self {
                FakeSupervisor::AllAvailable => Ok(vec![
                    status("gate", Liveness::Available),
                    status("daemon", Liveness::Available),
                ]),
                FakeSupervisor::OneUnavailable => Ok(vec![
                    status("gate", Liveness::Available),
                    status("daemon", Liveness::Unavailable),
                ]),
                FakeSupervisor::Errors => Err(OrchestratorError::ServiceUnavailable {
                    service: "daemon".to_string(),
                    reason: "spawn failed".to_string(),
                }),
            }
        }
    }

    fn open_ok() -> Result<Box<dyn Store>> {
        Ok(Box::new(InMemoryStore::new()))
    }

    fn open_err() -> Result<Box<dyn Store>> {
        Err(OrchestratorError::StoreVersionTooNew {
            found: 2,
            supported: 1,
        })
    }

    #[test]
    fn boot_reaches_ready_and_emits_transitions_in_order() {
        let sink = RecordingSink::default();
        let mut supervisor = FakeSupervisor::AllAvailable;

        let orch = boot(open_ok, &mut supervisor, &sink).unwrap();

        assert!(orch.is_ready());
        assert_eq!(
            sink.events(),
            vec![
                Status::Booting,
                Status::OpeningStore,
                Status::Supervising,
                Status::Ready,
            ]
        );
    }

    #[test]
    fn ready_not_reported_when_store_open_fails() {
        let sink = RecordingSink::default();
        let mut supervisor = FakeSupervisor::AllAvailable;

        let result = boot(open_err, &mut supervisor, &sink);

        assert!(matches!(
            result,
            Err(OrchestratorError::StoreVersionTooNew { .. })
        ));
        let events = sink.events();
        assert!(!events.contains(&Status::Ready), "must not report ready");
        assert!(matches!(events.last(), Some(Status::Failed { .. })));
    }

    #[test]
    fn ready_not_reported_when_a_service_is_unavailable() {
        let sink = RecordingSink::default();
        let mut supervisor = FakeSupervisor::OneUnavailable;

        let result = boot(open_ok, &mut supervisor, &sink);

        assert!(matches!(
            result,
            Err(OrchestratorError::ServiceUnavailable { .. })
        ));
        assert!(!sink.events().contains(&Status::Ready));
    }

    #[test]
    fn supervision_failure_surfaces_a_typed_error_and_no_ready() {
        let sink = RecordingSink::default();
        let mut supervisor = FakeSupervisor::Errors;

        let result = boot(open_ok, &mut supervisor, &sink);

        assert!(matches!(
            result,
            Err(OrchestratorError::ServiceUnavailable { .. })
        ));
        let events = sink.events();
        assert!(!events.contains(&Status::Ready));
        assert!(matches!(events.last(), Some(Status::Failed { .. })));
    }

    #[test]
    fn boot_yields_one_instance_that_owns_a_working_store() {
        let sink = RecordingSink::default();
        let mut supervisor = FakeSupervisor::AllAvailable;

        let orch = boot(open_ok, &mut supervisor, &sink).unwrap();

        assert_eq!(
            orch.store().schema_version().unwrap(),
            current_schema_version()
        );
        assert_eq!(orch.service_statuses().len(), 2);
    }
}
