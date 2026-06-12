//! Builds the app with the full `@tillerd/sdk` WorkspaceClient command set. The
//! `generate_handler!` listing compile-checks that every command exists and is registerable (a
//! removed/renamed command fails the build), and constructing the app validates the managed state
//! wiring — catching the "implemented but never registered" class (e.g. a missing `project_list`)
//! at unit-test speed instead of a full desktop e2e cycle. Per-command argument shapes are
//! exercised against the real app by the desktop-e2e smokes.

use crate::orchestrator_host::OrchestratorState;
use crate::surface_host::SurfaceState;
use crate::workspace_host;
use orchestrator::persistence::memory::InMemoryStore;
use orchestrator::persistence::{Store, SurfaceId};
use orchestrator::surface::{SurfaceApi, SurfaceEventSink};
use std::sync::Arc;
use tauri::test::{mock_builder, mock_context, noop_assets};
use tauri::Manager;

struct NullSink;
impl SurfaceEventSink for NullSink {
    fn on_bytes(&self, _: &SurfaceId, _: &[u8]) {}
    fn on_status(&self, _: &SurfaceId, _: &str) {}
    fn on_exit(&self, _: &SurfaceId, _: &str) {}
}

#[test]
fn every_workspace_client_command_is_registered() {
    let store: Arc<dyn Store> = Arc::new(InMemoryStore::new());
    let api = Arc::new(SurfaceApi::new(
        store,
        Arc::new(NullSink),
        "/tmp/tillerd-contract.sock".into(),
    ));
    let surfaces = SurfaceState {
        api,
        channels: Default::default(),
    };

    let app = mock_builder()
        .manage(OrchestratorState::default())
        .manage(surfaces)
        .invoke_handler(tauri::generate_handler![
            workspace_host::project_create,
            workspace_host::project_list,
            workspace_host::project_rename,
            workspace_host::project_archive,
            workspace_host::session_list,
            workspace_host::session_create,
            workspace_host::session_rename,
            workspace_host::session_archive,
            workspace_host::session_layout_set,
            workspace_host::session_layout_get,
            workspace_host::command_list,
            workspace_host::command_create,
            workspace_host::command_get,
            workspace_host::command_delete,
        ])
        .build(mock_context(noop_assets()))
        .expect("app builds with the full workspace command set + managed state");

    // The managed state the workspace commands resolve is wired.
    assert!(app.try_state::<OrchestratorState>().is_some());
    assert!(app.try_state::<SurfaceState>().is_some());
}
