//! Contract scenarios for the `store-architecture` delta spec: per-entity stores over a closed
//! `Backend` enum, behavior identical across backends, composition-root selection, and the
//! cross-aggregate `create_session` coordinator.

use orchestrator::entities::{
    NewLaunchTemplate, NewProject, NewSession, NewWorkspace, ProjectId, SettingScope, SourceKind,
};
use orchestrator::infra::fs::FsBackend;
use orchestrator::infra::memory::MemoryBackend;
use orchestrator::infra::sqlite::SqliteBackend;
use orchestrator::store::{create_session, ProjectFilter, Storage};
use tempfile::TempDir;

fn fs_storage() -> (TempDir, Storage) {
    let tmp = TempDir::new().unwrap();
    let fs = FsBackend::open(tmp.path().join("tree")).unwrap();
    let sqlite = SqliteBackend::open(&tmp.path().join("tillerd.db")).unwrap();
    (tmp, Storage::open(fs, sqlite))
}

fn memory_storage() -> Storage {
    Storage::in_memory(MemoryBackend::new())
}

/// The observable result of a domain create/list/archive round-trip, with the minted
/// (non-deterministic) ids stripped so two backends can be compared for equality.
#[derive(Debug, PartialEq)]
struct DomainOutcome {
    project_names_before_archive: Vec<String>,
    project_visible_after_archive: bool,
    session_belongs_to_created_project: bool,
}

async fn domain_round_trip(storage: &Storage) -> DomainOutcome {
    let workspace = storage
        .workspaces
        .create(NewWorkspace {
            name: "Acme".to_string(),
        })
        .await
        .unwrap();
    let project = storage
        .projects
        .create(NewProject {
            source_kind: SourceKind::Blank,
            root_path: None,
            name: Some("Web".to_string()),
            workspace_id: Some(workspace.id.clone()),
        })
        .await
        .unwrap();
    let session = storage
        .sessions
        .create(
            NewSession {
                project_id: Some(project.id.clone()),
                ..Default::default()
            },
            None,
        )
        .await
        .unwrap();

    let filter = ProjectFilter {
        workspace: Some(workspace.id.clone()),
    };
    let before = storage.projects.list(&filter).await.unwrap();
    let project_names_before_archive = before.iter().map(|p| p.name.clone()).collect();

    storage.projects.archive(project.id.clone()).await.unwrap();
    let project_visible_after_archive = storage
        .projects
        .list(&filter)
        .await
        .unwrap()
        .iter()
        .any(|p| p.id == project.id);

    DomainOutcome {
        project_names_before_archive,
        project_visible_after_archive,
        session_belongs_to_created_project: session.project_id == project.id,
    }
}

// Scenario: Round-trip identical across backends
#[tokio::test]
async fn domain_round_trip_is_identical_across_memory_and_fs() {
    let memory = memory_storage();
    let (_tmp, fs) = fs_storage();

    let memory_outcome = domain_round_trip(&memory).await;
    let fs_outcome = domain_round_trip(&fs).await;

    assert_eq!(memory_outcome, fs_outcome);
    assert_eq!(
        memory_outcome.project_names_before_archive,
        vec!["Web".to_string()]
    );
    assert!(!memory_outcome.project_visible_after_archive);
    assert!(memory_outcome.session_belongs_to_created_project);
}

// Scenario: A typed filter is pushed to the backend
#[tokio::test]
async fn project_filter_scopes_to_its_workspace() {
    let storage = memory_storage();
    let ws_a = storage
        .workspaces
        .create(NewWorkspace {
            name: "A".to_string(),
        })
        .await
        .unwrap();
    let ws_b = storage
        .workspaces
        .create(NewWorkspace {
            name: "B".to_string(),
        })
        .await
        .unwrap();
    let in_a = storage
        .projects
        .create(NewProject {
            source_kind: SourceKind::Blank,
            root_path: None,
            name: Some("InA".to_string()),
            workspace_id: Some(ws_a.id.clone()),
        })
        .await
        .unwrap();
    storage
        .projects
        .create(NewProject {
            source_kind: SourceKind::Blank,
            root_path: None,
            name: Some("InB".to_string()),
            workspace_id: Some(ws_b.id.clone()),
        })
        .await
        .unwrap();

    let scoped = storage
        .projects
        .list(&ProjectFilter {
            workspace: Some(ws_a.id),
        })
        .await
        .unwrap();
    assert_eq!(scoped.len(), 1);
    assert_eq!(scoped[0].id, in_a.id);
}

// Scenario: Operational entities go through their own stores (not a shared facade)
#[tokio::test]
async fn operational_entities_each_have_their_own_store() {
    let storage = memory_storage();

    assert!(
        !storage.commands.list().await.unwrap().is_empty(),
        "seeded commands are reachable through the Commands store"
    );

    storage
        .settings
        .set(
            SettingScope::Global,
            "theme".to_string(),
            "\"dark\"".to_string(),
        )
        .await
        .unwrap();
    assert_eq!(
        storage
            .settings
            .get(SettingScope::Global, "theme".to_string())
            .await
            .unwrap(),
        Some("\"dark\"".to_string())
    );

    assert!(storage.notifications.list(10).await.unwrap().is_empty());
}

// Scenario: Templated session creation resolves through the coordinator
#[tokio::test]
async fn templated_session_creation_resolves_through_coordinator() {
    let storage = memory_storage();
    let template = storage
        .launch_templates
        .create(NewLaunchTemplate {
            project_id: ProjectId::unfiled(),
            spec_version: 1,
            spec_json: r#"{"version":1,"items":[]}"#.to_string(),
        })
        .await
        .unwrap();

    let session = create_session(
        NewSession {
            template_id: Some(template.id),
            ..Default::default()
        },
        &storage.launch_templates,
        &storage.sessions,
    )
    .await
    .unwrap();

    let fetched = storage.sessions.get(session.id).await.unwrap().unwrap();
    assert_eq!(fetched.spec_version, Some(1));
}
