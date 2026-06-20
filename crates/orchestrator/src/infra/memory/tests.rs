use super::*;
use crate::entities::SurfaceKind;

#[test]
fn fake_reports_current_schema_version() {
    let store = MemoryBackend::new();
    assert_eq!(store.schema_version().unwrap(), current_version());
}

#[test]
fn fake_seeds_unfiled_and_resolves_sessions_to_it() {
    let store = MemoryBackend::new();
    assert!(store.get_project(&ProjectId::unfiled()).unwrap().is_some());

    let session = store
        .create_session_inner(NewSession::default(), None)
        .unwrap();
    assert_eq!(session.project_id, ProjectId::unfiled());
}

fn make_surface(store: &MemoryBackend) -> Surface {
    let session = store
        .create_session_inner(NewSession::default(), None)
        .unwrap();
    store
        .create_surface(NewSurface {
            id: None,
            session_id: session.id,
            kind: SurfaceKind::Terminal,
            cwd: None,
            placement: None,
        })
        .unwrap()
}

#[test]
fn create_then_get_surface_round_trips_including_last_status_none() {
    let store = MemoryBackend::new();

    let created = make_surface(&store);
    let fetched = store.get_surface(&created.id).unwrap().unwrap();

    assert_eq!(fetched, created);
    assert!(fetched.last_status.is_none());
}

#[test]
fn list_resumable_surfaces_includes_a_created_surface() {
    let store = MemoryBackend::new();

    let created = make_surface(&store);
    let list = store.list_resumable_surfaces().unwrap();

    assert!(list.iter().any(|s| s.id == created.id));
}

#[test]
fn soft_delete_excludes_surface_from_list_and_get() {
    let store = MemoryBackend::new();

    let surface = make_surface(&store);
    store.soft_delete_surface(&surface.id).unwrap();

    assert!(store.get_surface(&surface.id).unwrap().is_none());
    let list = store.list_resumable_surfaces().unwrap();
    assert!(!list.iter().any(|s| s.id == surface.id));
}

#[test]
fn update_surface_status_is_reflected_by_get_surface() {
    let store = MemoryBackend::new();

    let surface = make_surface(&store);
    store.update_surface_status(&surface.id, "running").unwrap();

    let fetched = store.get_surface(&surface.id).unwrap().unwrap();
    assert_eq!(fetched.last_status.as_deref(), Some("running"));
}

#[test]
fn set_launch_template_spec_on_absent_template_is_not_found() {
    let store = MemoryBackend::new();
    let err = store
        .set_launch_template_spec(
            &LaunchTemplateId::from_string("no-such-template"),
            2,
            r#"{"version":2,"items":[]}"#,
        )
        .unwrap_err();
    assert!(matches!(err, OrchestratorError::LaunchTemplateNotFound(_)));
}

// ── settings ──────────────────────────────────────────────────────────

#[test]
fn global_setting_round_trips() {
    let store = MemoryBackend::new();
    store
        .set_setting(&SettingScope::Global, "theme", r#""dark""#)
        .unwrap();
    let got = store.get_setting(&SettingScope::Global, "theme").unwrap();
    assert_eq!(got.as_deref(), Some(r#""dark""#));
}

#[test]
fn project_scoped_setting_round_trips() {
    let store = MemoryBackend::new();
    let scope = SettingScope::Project(ProjectId::unfiled());
    store
        .set_setting(&scope, "env", r#"{"FOO":"bar"}"#)
        .unwrap();
    let got = store.get_setting(&scope, "env").unwrap();
    assert_eq!(got.as_deref(), Some(r#"{"FOO":"bar"}"#));
    // The same key under global is independent.
    assert!(store
        .get_setting(&SettingScope::Global, "env")
        .unwrap()
        .is_none());
}

#[test]
fn overwriting_a_setting_replaces_the_value() {
    let store = MemoryBackend::new();
    store
        .set_setting(&SettingScope::Global, "k", r#"1"#)
        .unwrap();
    store
        .set_setting(&SettingScope::Global, "k", r#"2"#)
        .unwrap();
    assert_eq!(
        store
            .get_setting(&SettingScope::Global, "k")
            .unwrap()
            .as_deref(),
        Some("2")
    );
    // No duplicate rows: exactly one entry for the key.
    let listed = store.list_settings(&SettingScope::Global).unwrap();
    assert_eq!(listed.iter().filter(|e| e.key == "k").count(), 1);
}

#[test]
fn project_value_takes_precedence_over_global() {
    let store = MemoryBackend::new();
    let pid = ProjectId::unfiled();
    store
        .set_setting(&SettingScope::Global, "template", r#""g""#)
        .unwrap();
    store
        .set_setting(&SettingScope::Project(pid.clone()), "template", r#""p""#)
        .unwrap();
    let resolved = store.resolve_setting(&pid, "template").unwrap();
    assert_eq!(resolved.as_deref(), Some(r#""p""#));
}

#[test]
fn resolve_falls_back_to_global_on_project_miss() {
    let store = MemoryBackend::new();
    let pid = ProjectId::unfiled();
    store
        .set_setting(&SettingScope::Global, "template", r#""g""#)
        .unwrap();
    let resolved = store.resolve_setting(&pid, "template").unwrap();
    assert_eq!(resolved.as_deref(), Some(r#""g""#));
}

#[test]
fn unknown_key_resolves_to_absent() {
    let store = MemoryBackend::new();
    let resolved = store
        .resolve_setting(&ProjectId::unfiled(), "never-set")
        .unwrap();
    assert!(resolved.is_none());
}

#[test]
fn list_returns_written_entries() {
    let store = MemoryBackend::new();
    store.set_setting(&SettingScope::Global, "a", "1").unwrap();
    store.set_setting(&SettingScope::Global, "b", "2").unwrap();
    let listed = store.list_settings(&SettingScope::Global).unwrap();
    assert_eq!(
        listed,
        vec![
            SettingEntry {
                key: "a".to_string(),
                value_json: "1".to_string()
            },
            SettingEntry {
                key: "b".to_string(),
                value_json: "2".to_string()
            },
        ]
    );
}

#[test]
fn suppression_choice_is_recorded_and_read() {
    let store = MemoryBackend::new();
    store
        .set_setting(&SettingScope::Global, "confirm.close-surface", "true")
        .unwrap();
    let got = store
        .get_setting(&SettingScope::Global, "confirm.close-surface")
        .unwrap();
    assert_eq!(got.as_deref(), Some("true"));
}

#[test]
fn unset_confirmation_reads_as_absent() {
    let store = MemoryBackend::new();
    let got = store
        .get_setting(&SettingScope::Global, "confirm.never-shown")
        .unwrap();
    assert!(got.is_none(), "absent confirmation is not suppressed");
}
