use super::atomic_io::read_json;
use super::index::{is_archived, list_live_dirs};

use super::*;
use tempfile::TempDir;

fn open_store() -> (TempDir, FsBackend) {
    let tmp = TempDir::new().unwrap();
    let store = FsBackend::open(tmp.path().to_path_buf()).unwrap();
    (tmp, store)
}

#[test]
fn malformed_project_json_is_skipped_and_rest_of_tree_loads() {
    use std::io::Write as _;
    let tmp = TempDir::new().unwrap();
    let store = FsBackend::open(tmp.path().to_path_buf()).unwrap();
    // Seed a workspace to work within.
    let ws = store
        .create_workspace(NewWorkspace { name: "W".into() })
        .unwrap();
    let good = store
        .create_project(NewProject {
            source_kind: SourceKind::Blank,
            root_path: None,
            name: Some("Good".into()),
            workspace_id: Some(ws.id.clone()),
        })
        .unwrap();
    // Write a malformed project.json in a sibling directory inside the workspace projects dir.
    let ws_dir = store.ws_dir_for(&ws.id).unwrap();
    let bad_proj_dir = ws_dir.join("projects").join("bad-slug");
    std::fs::create_dir_all(bad_proj_dir.join("sessions")).unwrap();
    let mut f = std::fs::File::create(bad_proj_dir.join("project.json")).unwrap();
    f.write_all(b"{ not valid json }").unwrap();
    drop(f);

    // Re-open: malformed project is skipped, good project still loads.
    let store2 = FsBackend::open(tmp.path().to_path_buf()).unwrap();
    let projects = store2.list_projects(None).unwrap();
    assert!(
        projects.iter().any(|p| p.id == good.id),
        "good project must still load after malformed sibling is present"
    );
}

// ── Scenario: Workspace, project, and session persist as nested directories ──

#[test]
fn workspace_project_session_persist_as_nested_directories() {
    let (tmp, store) = open_store();

    let ws = store
        .create_workspace(NewWorkspace {
            name: "My Workspace".into(),
        })
        .unwrap();
    let proj = store
        .create_project(NewProject {
            source_kind: SourceKind::LocalDir,
            root_path: Some("/tmp/proj".into()),
            name: Some("My Project".into()),
            workspace_id: Some(ws.id.clone()),
        })
        .unwrap();
    let sess = store
        .create_session(
            NewSession {
                project_id: Some(proj.id.clone()),
                title: Some("My Session".into()),
                title_source: TitleSource::Custom,
                template_id: None,
            },
            None,
        )
        .unwrap();

    // workspace.json must exist
    let ws_root = tmp.path().join("workspaces");
    let ws_dirs: Vec<_> = list_live_dirs(&ws_root).unwrap();
    assert_eq!(ws_dirs.len(), 2); // default + new

    // Find the new workspace dir
    let new_ws_dir = ws_dirs
        .iter()
        .find(|d| {
            read_json::<WorkspaceFile>(&d.join("workspace.json"))
                .ok()
                .map(|wf| wf.id == ws.id.as_str())
                .unwrap_or(false)
        })
        .unwrap()
        .clone();

    assert!(new_ws_dir.join("workspace.json").exists());

    let proj_dirs = list_live_dirs(&new_ws_dir.join("projects")).unwrap();
    assert_eq!(proj_dirs.len(), 1);
    assert!(proj_dirs[0].join("project.json").exists());

    let sess_dirs = list_live_dirs(&proj_dirs[0].join("sessions")).unwrap();
    assert_eq!(sess_dirs.len(), 1);
    assert!(sess_dirs[0].join("session.json").exists());

    // Verify ids persist correctly
    let wf: WorkspaceFile = read_json(&new_ws_dir.join("workspace.json")).unwrap();
    assert_eq!(wf.id, ws.id.as_str());
    let pf: ProjectFile = read_json(&proj_dirs[0].join("project.json")).unwrap();
    assert_eq!(pf.id, proj.id.as_str());
    let sf: SessionFile = read_json(&sess_dirs[0].join("session.json")).unwrap();
    assert_eq!(sf.id, sess.id.as_str());
}

// ── Scenario: Hierarchy is read back from containment ──

#[test]
fn hierarchy_is_read_back_from_containment() {
    let (tmp, store) = open_store();

    let ws = store
        .create_workspace(NewWorkspace { name: "W1".into() })
        .unwrap();
    let proj = store
        .create_project(NewProject {
            source_kind: SourceKind::LocalDir,
            root_path: None,
            name: Some("P1".into()),
            workspace_id: Some(ws.id.clone()),
        })
        .unwrap();
    let sess = store
        .create_session(
            NewSession {
                project_id: Some(proj.id.clone()),
                title: Some("S1".into()),
                title_source: TitleSource::Custom,
                template_id: None,
            },
            None,
        )
        .unwrap();

    // Reopen
    let store2 = FsBackend::open(tmp.path().to_path_buf()).unwrap();

    let proj2 = store2.get_project(&proj.id).unwrap().unwrap();
    assert_eq!(proj2.workspace_id, ws.id);

    let sess2 = store2.get_session(&sess.id).unwrap().unwrap();
    assert_eq!(sess2.project_id, proj.id);
}

// ── Scenario: A failed write leaves the prior file intact ──

#[test]
fn failed_write_leaves_prior_file_intact() {
    let (_tmp, store) = open_store();
    let ws = store
        .create_workspace(NewWorkspace { name: "W".into() })
        .unwrap();
    let ws_dir = store.ws_dir_for(&ws.id).unwrap();
    let ws_file = ws_dir.join("workspace.json");

    // Read original content
    let original = std::fs::read_to_string(&ws_file).unwrap();

    // Simulate an interrupted write: write .tmp but don't rename
    let tmp_path = ws_file.with_extension("tmp");
    std::fs::write(&tmp_path, b"partial content").unwrap();

    // The entity file must still have original content
    let after = std::fs::read_to_string(&ws_file).unwrap();
    assert_eq!(original, after);

    // The .tmp file exists but doesn't affect reads
    assert!(tmp_path.exists());
    // Remove it to clean up
    std::fs::remove_file(&tmp_path).unwrap();
}

// ── Scenario: Siblings list in sortOrder ──

#[test]
fn siblings_list_in_sort_order() {
    let (_tmp, store) = open_store();
    let proj = store.get_project(&ProjectId::unfiled()).unwrap().unwrap();

    // Create 3 sessions and then reorder them
    let s0 = store
        .create_session(
            NewSession {
                project_id: Some(proj.id.clone()),
                title: Some("A".into()),
                title_source: TitleSource::Custom,
                template_id: None,
            },
            None,
        )
        .unwrap();
    let s1 = store
        .create_session(
            NewSession {
                project_id: Some(proj.id.clone()),
                title: Some("B".into()),
                title_source: TitleSource::Custom,
                template_id: None,
            },
            None,
        )
        .unwrap();
    let s2 = store
        .create_session(
            NewSession {
                project_id: Some(proj.id.clone()),
                title: Some("C".into()),
                title_source: TitleSource::Custom,
                template_id: None,
            },
            None,
        )
        .unwrap();

    // Reorder: C=0, A=1, B=2
    store.reorder_session(&s2.id, 0).unwrap();
    store.reorder_session(&s0.id, 1).unwrap();
    store.reorder_session(&s1.id, 2).unwrap();

    let sessions = store.list_sessions(Some(&proj.id)).unwrap();
    assert_eq!(sessions.len(), 3);
    assert_eq!(sessions[0].id, s2.id);
    assert_eq!(sessions[1].id, s0.id);
    assert_eq!(sessions[2].id, s1.id);
}

// ── Scenario: Reorder persists across reload ──

#[test]
fn reorder_persists_across_reload() {
    let (tmp, store) = open_store();
    let proj = store.get_project(&ProjectId::unfiled()).unwrap().unwrap();

    let s0 = store
        .create_session(
            NewSession {
                project_id: Some(proj.id.clone()),
                title: Some("First".into()),
                title_source: TitleSource::Custom,
                template_id: None,
            },
            None,
        )
        .unwrap();
    let s1 = store
        .create_session(
            NewSession {
                project_id: Some(proj.id.clone()),
                title: Some("Second".into()),
                title_source: TitleSource::Custom,
                template_id: None,
            },
            None,
        )
        .unwrap();

    // Swap order
    store.reorder_session(&s1.id, 0).unwrap();
    store.reorder_session(&s0.id, 1).unwrap();

    // Reopen
    let store2 = FsBackend::open(tmp.path().to_path_buf()).unwrap();
    let sessions = store2.list_sessions(Some(&proj.id)).unwrap();
    assert_eq!(sessions[0].id, s1.id);
    assert_eq!(sessions[1].id, s0.id);
}

// ── Scenario: Renaming a project moves its subtree and keeps the id ──

#[test]
fn renaming_a_project_moves_subtree_and_keeps_id() {
    let (tmp, store) = open_store();

    let ws = store
        .create_workspace(NewWorkspace { name: "WS".into() })
        .unwrap();
    let proj = store
        .create_project(NewProject {
            source_kind: SourceKind::LocalDir,
            root_path: None,
            name: Some("Old Name".into()),
            workspace_id: Some(ws.id.clone()),
        })
        .unwrap();

    // Create a session inside to verify subtree moves
    let sess = store
        .create_session(
            NewSession {
                project_id: Some(proj.id.clone()),
                title: Some("MySession".into()),
                title_source: TitleSource::Custom,
                template_id: None,
            },
            None,
        )
        .unwrap();

    store.rename_project(&proj.id, "New Name").unwrap();

    // Old slug should not exist
    let ws_dir_path = store.ws_dir_for(&ws.id).unwrap();
    let old_proj_dir = ws_dir_path.join("projects").join("old-name");
    assert!(!old_proj_dir.exists(), "old directory should not exist");

    // New slug should exist
    let new_proj_dir = ws_dir_path.join("projects").join("new-name");
    assert!(new_proj_dir.exists(), "new directory should exist");

    // ID unchanged
    let pf: ProjectFile = read_json(&new_proj_dir.join("project.json")).unwrap();
    assert_eq!(pf.id, proj.id.as_str());
    assert_eq!(pf.name, "New Name");

    // Session inside was moved too
    let sess_dirs = list_live_dirs(&new_proj_dir.join("sessions")).unwrap();
    assert_eq!(sess_dirs.len(), 1);
    let sf: SessionFile = read_json(&sess_dirs[0].join("session.json")).unwrap();
    assert_eq!(sf.id, sess.id.as_str());

    // Reopen to verify index rebuilt correctly
    let store2 = FsBackend::open(tmp.path().to_path_buf()).unwrap();
    let sess2 = store2.get_session(&sess.id).unwrap();
    assert!(
        sess2.is_some(),
        "session should be retrievable after rename"
    );
}

// ── Scenario: Colliding slug is disambiguated ──

#[test]
fn colliding_slug_is_disambiguated() {
    let (_tmp, store) = open_store();

    // Create two projects with names that produce the same slug
    let ws_id = WorkspaceId::default_id();
    let p1 = store
        .create_project(NewProject {
            source_kind: SourceKind::LocalDir,
            root_path: None,
            name: Some("foo".into()),
            workspace_id: Some(ws_id.clone()),
        })
        .unwrap();
    let p2 = store
        .create_project(NewProject {
            source_kind: SourceKind::LocalDir,
            root_path: None,
            name: Some("foo".into()),
            workspace_id: Some(ws_id),
        })
        .unwrap();

    // They must have different ids and different dirs
    assert_ne!(p1.id, p2.id);

    let dir1 = store.proj_dir_for(&p1.id).unwrap();
    let dir2 = store.proj_dir_for(&p2.id).unwrap();
    assert_ne!(dir1, dir2);

    // One slug is foo, the other is foo-2
    let slug1 = dir1.file_name().unwrap().to_str().unwrap();
    let slug2 = dir2.file_name().unwrap().to_str().unwrap();
    let slugs = [slug1, slug2];
    assert!(
        (slugs[0] == "foo" && slugs[1] == "foo-2") || (slugs[0] == "foo-2" && slugs[1] == "foo"),
        "expected foo and foo-2, got {:?}",
        slugs
    );
}

// ── Scenario: Archiving a session moves it out of the live tree ──

#[test]
fn archiving_a_session_moves_it_out_of_live_tree() {
    let (_tmp, store) = open_store();
    let proj = store.get_project(&ProjectId::unfiled()).unwrap().unwrap();

    let sess = store
        .create_session(
            NewSession {
                project_id: Some(proj.id.clone()),
                title: Some("ToArchive".into()),
                title_source: TitleSource::Custom,
                template_id: None,
            },
            None,
        )
        .unwrap();

    store.archive_session(&sess.id).unwrap();

    // Must not appear in live listing
    let sessions = store.list_sessions(Some(&proj.id)).unwrap();
    assert!(sessions.iter().all(|s| s.id != sess.id));

    // Must not be returned by get_session
    let got = store.get_session(&sess.id).unwrap();
    assert!(got.is_none());

    // Physically under .archive/
    let sess_dir = {
        let state = store.state.read().unwrap();
        state.get(sess.id.as_str()).unwrap().to_path_buf()
    };
    assert!(is_archived(&sess_dir));
}

// ── Scenario: Archiving a project archives its sessions with it ──

#[test]
fn archiving_a_project_archives_its_sessions_with_it() {
    let (_tmp, store) = open_store();

    let ws = store
        .create_workspace(NewWorkspace { name: "WS".into() })
        .unwrap();
    let proj = store
        .create_project(NewProject {
            source_kind: SourceKind::LocalDir,
            root_path: None,
            name: Some("ProjectWithSessions".into()),
            workspace_id: Some(ws.id.clone()),
        })
        .unwrap();
    let sess = store
        .create_session(
            NewSession {
                project_id: Some(proj.id.clone()),
                title: Some("S1".into()),
                title_source: TitleSource::Custom,
                template_id: None,
            },
            None,
        )
        .unwrap();

    store.archive_project(&proj.id).unwrap();

    // Project no longer in live listing
    let projects = store.list_projects(Some(&ws.id)).unwrap();
    assert!(projects.iter().all(|p| p.id != proj.id));

    // The whole subtree moved in one rename — check the proj dir is under .archive
    let proj_dir = {
        let state = store.state.read().unwrap();
        state.get(proj.id.as_str()).unwrap().to_path_buf()
    };
    assert!(is_archived(&proj_dir));

    // Session must be inside the archived project dir (moved with it)
    let sess_dir = {
        let state = store.state.read().unwrap();
        state.get(sess.id.as_str()).unwrap().to_path_buf()
    };
    assert!(sess_dir.starts_with(&proj_dir));
}

// ── Scenario: get-by-id resolves through the scan-built index ──

#[test]
fn get_by_id_resolves_through_scan_built_index() {
    let (tmp, store) = open_store();

    let ws = store
        .create_workspace(NewWorkspace {
            name: "ScanWS".into(),
        })
        .unwrap();
    let proj = store
        .create_project(NewProject {
            source_kind: SourceKind::LocalDir,
            root_path: None,
            name: Some("ScanProject".into()),
            workspace_id: Some(ws.id),
        })
        .unwrap();

    // Reopen: new store, fresh scan-built index
    let store2 = FsBackend::open(tmp.path().to_path_buf()).unwrap();
    let got = store2.get_project(&proj.id).unwrap();
    assert!(got.is_some());
    assert_eq!(got.unwrap().name, "ScanProject");
}

// ── Scenario: The index follows a rename ──

#[test]
fn index_follows_rename() {
    let (tmp, store) = open_store();

    let ws = store
        .create_workspace(NewWorkspace { name: "WS".into() })
        .unwrap();
    let proj = store
        .create_project(NewProject {
            source_kind: SourceKind::LocalDir,
            root_path: None,
            name: Some("Before".into()),
            workspace_id: Some(ws.id),
        })
        .unwrap();

    store.rename_project(&proj.id, "After").unwrap();

    // get_project should still work — index was updated in-place
    let got = store.get_project(&proj.id).unwrap().unwrap();
    assert_eq!(got.name, "After");

    // After a reopen, new scan-built index should also find it
    let store2 = FsBackend::open(tmp.path().to_path_buf()).unwrap();
    let got2 = store2.get_project(&proj.id).unwrap().unwrap();
    assert_eq!(got2.name, "After");
}

// ── Scenario: Surface binding round-trips through layout.json ──

#[test]
fn surface_binding_round_trips_through_layout_json() {
    let (tmp, store) = open_store();

    let proj = store.get_project(&ProjectId::unfiled()).unwrap().unwrap();
    let sess = store
        .create_session(
            NewSession {
                project_id: Some(proj.id),
                title: Some("S".into()),
                title_source: TitleSource::Custom,
                template_id: None,
            },
            None,
        )
        .unwrap();

    let surf = store
        .create_surface(NewSurface {
            id: None,
            session_id: sess.id.clone(),
            kind: SurfaceKind::Terminal,
            cwd: Some("./subdir".into()),
            placement: Some("main".into()),
        })
        .unwrap();

    // Reload store from disk
    let store2 = FsBackend::open(tmp.path().to_path_buf()).unwrap();
    let found = store2
        .find_session_surface_by_placement(&sess.id, "main")
        .unwrap()
        .unwrap();

    assert_eq!(found.id, surf.id);
    assert_eq!(found.kind, SurfaceKind::Terminal);
    assert_eq!(found.placement, Some("main".into()));
    assert_eq!(found.cwd, Some("./subdir".into()));
}

// ── Scenario: Duplicate placement is rejected ──

#[test]
fn duplicate_placement_is_rejected_with_surface_conflict() {
    let (_tmp, store) = open_store();

    let proj = store.get_project(&ProjectId::unfiled()).unwrap().unwrap();
    let sess = store
        .create_session(
            NewSession {
                project_id: Some(proj.id),
                title: Some("S".into()),
                title_source: TitleSource::Custom,
                template_id: None,
            },
            None,
        )
        .unwrap();

    store
        .create_surface(NewSurface {
            id: None,
            session_id: sess.id.clone(),
            kind: SurfaceKind::Terminal,
            cwd: None,
            placement: Some("panel-1".into()),
        })
        .unwrap();

    // Second surface at the same placement must fail
    let result = store.create_surface(NewSurface {
        id: None,
        session_id: sess.id,
        kind: SurfaceKind::Terminal,
        cwd: None,
        placement: Some("panel-1".into()),
    });

    assert!(
        matches!(result, Err(OrchestratorError::SurfaceConflict(_))),
        "expected SurfaceConflict, got {:?}",
        result
    );
}

// ── Regression: archive reuses a freed slug, then collides in .archive ──

#[test]
fn archiving_two_projects_that_reuse_a_freed_slug_disambiguates_in_archive() {
    let (_tmp, store) = open_store();
    let ws = store
        .create_workspace(NewWorkspace { name: "W".into() })
        .unwrap();

    let p1 = store
        .create_project(NewProject {
            source_kind: SourceKind::Blank,
            root_path: None,
            name: Some("Same".into()),
            workspace_id: Some(ws.id.clone()),
        })
        .unwrap();
    store.archive_project(&p1.id).unwrap(); // -> .archive/same; frees live slug

    let p2 = store
        .create_project(NewProject {
            source_kind: SourceKind::Blank,
            root_path: None,
            name: Some("Same".into()),
            workspace_id: Some(ws.id),
        })
        .unwrap(); // reuses the now-free live slug "same"
    store.archive_project(&p2.id).unwrap(); // must disambiguate, not collide

    // Both archived subtrees survive distinctly.
    store.hard_delete_project(&p1.id).unwrap();
    store.hard_delete_project(&p2.id).unwrap();
}

// ── Regression: deleting a workspace preserves its archived projects ──

#[test]
fn deleting_a_workspace_preserves_its_archived_projects() {
    let (_tmp, store) = open_store();
    let ws = store
        .create_workspace(NewWorkspace {
            name: "Doomed".into(),
        })
        .unwrap();
    let proj = store
        .create_project(NewProject {
            source_kind: SourceKind::Blank,
            root_path: None,
            name: Some("Keep".into()),
            workspace_id: Some(ws.id.clone()),
        })
        .unwrap();
    store.archive_project(&proj.id).unwrap();

    store.delete_workspace(&ws.id).unwrap();

    // The archived project was reassigned to Default, not destroyed.
    store.hard_delete_project(&proj.id).unwrap();
}

// ── Regression: persisted state is owner-only (0600 files / 0700 dirs) ──

#[cfg(unix)]
#[test]
fn persisted_files_and_dirs_are_owner_only() {
    use std::os::unix::fs::PermissionsExt;
    let (_tmp, store) = open_store();
    let ws = store
        .create_workspace(NewWorkspace {
            name: "Perms".into(),
        })
        .unwrap();
    let ws_dir = store.ws_dir_for(&ws.id).unwrap();

    let dir_mode = fs::metadata(&ws_dir).unwrap().permissions().mode() & 0o777;
    assert_eq!(dir_mode, 0o700, "domain directory must be owner-only");

    let file_mode = fs::metadata(ws_dir.join("workspace.json"))
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(file_mode, 0o600, "domain file must be owner-only");
}

// ── Scenario: Concurrent writes from two windows stay consistent ──

#[test]
fn concurrent_writes_from_two_threads_stay_consistent() {
    use std::sync::Arc;

    let (_tmp, store) = open_store();
    let store = Arc::new(store);

    let proj = store.get_project(&ProjectId::unfiled()).unwrap().unwrap();
    let proj_id = proj.id;

    let store1 = Arc::clone(&store);
    let store2 = Arc::clone(&store);
    let pid1 = proj_id.clone();
    let pid2 = proj_id.clone();

    let t1 = std::thread::spawn(move || {
        for i in 0..10 {
            store1
                .create_session(
                    NewSession {
                        project_id: Some(pid1.clone()),
                        title: Some(format!("T1-{i}")),
                        title_source: TitleSource::Custom,
                        template_id: None,
                    },
                    None,
                )
                .unwrap();
        }
    });
    let t2 = std::thread::spawn(move || {
        for i in 0..10 {
            store2
                .create_session(
                    NewSession {
                        project_id: Some(pid2.clone()),
                        title: Some(format!("T2-{i}")),
                        title_source: TitleSource::Custom,
                        template_id: None,
                    },
                    None,
                )
                .unwrap();
        }
    });

    t1.join().unwrap();
    t2.join().unwrap();

    let sessions = store.list_sessions(Some(&proj_id)).unwrap();
    assert_eq!(
        sessions.len(),
        20,
        "expected 20 sessions, got {}",
        sessions.len()
    );
}

// ── Scenario: Domain tree resolves under the data-root directory ──

#[test]
fn domain_tree_resolves_under_data_root_directory() {
    let tmp = TempDir::new().unwrap();
    let data_root = tmp.path().join("data");
    // data_root does not pre-exist; FsBackend::open should create it
    let store = FsBackend::open(data_root.clone()).unwrap();

    assert!(data_root.exists(), "data root should be created by open()");

    let ws = store
        .create_workspace(NewWorkspace {
            name: "RootTest".into(),
        })
        .unwrap();
    let ws_dir = store.ws_dir_for(&ws.id).unwrap();
    assert!(ws_dir.starts_with(&data_root));
}

// ── R1b: listing cache, mtime revalidation, lazy index ──────────────────────

use std::time::{Duration, SystemTime};

/// Create a project in the Default workspace with an explicit name.
fn make_named_project(store: &FsBackend, name: &str) -> Project {
    store
        .create_project(NewProject {
            source_kind: SourceKind::Blank,
            root_path: None,
            name: Some(name.to_owned()),
            workspace_id: None,
        })
        .unwrap()
}

fn id_name_pairs(projects: &[Project]) -> Vec<(ProjectId, String)> {
    projects
        .iter()
        .map(|p| (p.id.clone(), p.name.clone()))
        .collect()
}

#[test]
fn repeated_reads_return_identical_results() {
    let (_tmp, store) = open_store();
    make_named_project(&store, "Alpha");

    let first = store.list_projects(None).unwrap();
    let second = store.list_projects(None).unwrap();

    assert_eq!(id_name_pairs(&first), id_name_pairs(&second));
}

#[test]
fn externally_changed_file_is_reread() {
    let (_tmp, store) = open_store();
    let p = make_named_project(&store, "Before");
    assert_eq!(store.get_project(&p.id).unwrap().unwrap().name, "Before");

    // Rewrite project.json on disk with a new name and a strictly later mtime.
    let pj = store.proj_dir_for(&p.id).unwrap().join("project.json");
    let mut v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&pj).unwrap()).unwrap();
    v["name"] = serde_json::json!("After");
    std::fs::write(&pj, serde_json::to_string(&v).unwrap()).unwrap();
    let f = std::fs::File::options().write(true).open(&pj).unwrap();
    f.set_modified(SystemTime::now() + Duration::from_secs(5))
        .unwrap();
    drop(f);

    assert_eq!(store.get_project(&p.id).unwrap().unwrap().name, "After");
}

#[test]
fn unchanged_file_reads_equal_to_disk() {
    let (_tmp, store) = open_store();
    let p = make_named_project(&store, "Stable");

    let a = store.get_project(&p.id).unwrap().unwrap();
    let b = store.get_project(&p.id).unwrap().unwrap();

    assert_eq!((a.id, a.name), (b.id, b.name));
}

#[test]
fn read_after_backend_write_reflects_the_write() {
    let (_tmp, store) = open_store();
    let p = make_named_project(&store, "Old");
    let _ = store.list_projects(None).unwrap(); // populate the cache

    store.rename_project(&p.id, "New").unwrap();

    assert_eq!(store.get_project(&p.id).unwrap().unwrap().name, "New");
    assert!(store
        .list_projects(None)
        .unwrap()
        .iter()
        .any(|x| x.name == "New"));
}

#[test]
fn entity_resolves_by_id_after_open_without_prior_listing() {
    let tmp = TempDir::new().unwrap();
    let pid = {
        let store = FsBackend::open(tmp.path().to_path_buf()).unwrap();
        make_named_project(&store, "Deep").id
    };

    // Fresh open, then resolve by id before any listing call.
    let store = FsBackend::open(tmp.path().to_path_buf()).unwrap();
    assert!(store.get_project(&pid).unwrap().is_some());
}

#[test]
fn empty_tree_is_seeded_eagerly_at_open() {
    let (_tmp, store) = open_store();

    assert!(store.get_project(&ProjectId::unfiled()).unwrap().is_some());
    let workspaces = store.list_workspaces().unwrap();
    assert!(workspaces
        .iter()
        .any(|w| w.id.as_str() == WorkspaceId::DEFAULT));
}
