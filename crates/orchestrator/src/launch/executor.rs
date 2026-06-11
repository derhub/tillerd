use std::sync::Arc;

use crate::error::Result;
use crate::launch::spec::{CommandRef, LaunchItem, LaunchSpec};
use crate::persistence::{NewSurface, SessionId, Store, SurfaceKind};

/// Outcome for a single launch item.
#[derive(Debug)]
pub struct LaunchItemResult {
    pub index: usize,
    pub error: Option<String>,
}

/// Execute all items in `spec` in order against the given session.
/// Uses best-effort: a failed item is recorded and execution continues.
pub fn run(
    spec: &LaunchSpec,
    session_id: &SessionId,
    store: &Arc<dyn Store>,
) -> Vec<LaunchItemResult> {
    spec.items
        .iter()
        .enumerate()
        .map(|(i, item)| run_item(i, item, session_id, store))
        .collect()
}

fn run_item(
    index: usize,
    item: &LaunchItem,
    session_id: &SessionId,
    store: &Arc<dyn Store>,
) -> LaunchItemResult {
    match try_run_item(item, session_id, store) {
        Ok(()) => LaunchItemResult { index, error: None },
        Err(e) => LaunchItemResult {
            index,
            error: Some(e.to_string()),
        },
    }
}

fn try_run_item(item: &LaunchItem, session_id: &SessionId, store: &Arc<dyn Store>) -> Result<()> {
    // Resolve command (fail fast on unknown library ref — error surfaced on item)
    let cwd = match &item.command {
        CommandRef::LibraryRef { library_ref } => match store.get_command(library_ref)? {
            Some(_) => None,
            None => {
                return Err(crate::error::OrchestratorError::CommandNotFound(
                    library_ref.clone(),
                ))
            }
        },
        CommandRef::Inline { .. } => None,
    };

    store.create_surface(NewSurface {
        id: None,
        session_id: session_id.clone(),
        kind: SurfaceKind::Terminal,
        cwd,
        placement: item.placement.clone(),
        worktree_id: None,
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::launch::spec::{CommandRef, LaunchItem, LaunchSpec};
    use crate::persistence::memory::InMemoryStore;
    use crate::persistence::NewSession;

    fn make_store() -> Arc<dyn Store> {
        Arc::new(InMemoryStore::new())
    }

    fn make_session(store: &Arc<dyn Store>) -> SessionId {
        store.create_session(NewSession::default()).unwrap().id
    }

    fn item_with_inline(placement: Option<&str>) -> LaunchItem {
        LaunchItem {
            target: "terminal".to_string(),
            placement: placement.map(str::to_string),
            command: CommandRef::Inline {
                executable: "/bin/sh".to_string(),
                args: vec![],
            },
            pre: vec![],
            post: vec![],
            auto_spawn: vec![],
            worktree: None,
        }
    }

    #[test]
    fn items_run_in_list_order() {
        let store = make_store();
        let session_id = make_session(&store);
        let spec = LaunchSpec {
            version: 1,
            items: vec![item_with_inline(None), item_with_inline(None)],
        };

        let results = run(&spec, &session_id, &store);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].index, 0);
        assert_eq!(results[1].index, 1);
        assert!(results[0].error.is_none());
        assert!(results[1].error.is_none());
    }

    #[test]
    fn failed_item_does_not_block_subsequent_items() {
        let store = make_store();
        let session_id = make_session(&store);
        // Item 0 uses unknown library_ref — should fail
        // Item 1 uses inline — should succeed
        let spec = LaunchSpec {
            version: 1,
            items: vec![
                LaunchItem {
                    target: "terminal".to_string(),
                    placement: None,
                    command: CommandRef::LibraryRef {
                        library_ref: "no-such-command".to_string(),
                    },
                    pre: vec![],
                    post: vec![],
                    auto_spawn: vec![],
                    worktree: None,
                },
                item_with_inline(None),
            ],
        };

        let results = run(&spec, &session_id, &store);

        assert_eq!(results.len(), 2);
        assert!(
            results[0].error.is_some(),
            "unknown library_ref should fail"
        );
        assert!(results[1].error.is_none(), "second item should succeed");
    }

    #[test]
    fn placement_hint_stored_when_present() {
        let store = make_store();
        let session_id = make_session(&store);
        let spec = LaunchSpec {
            version: 1,
            items: vec![item_with_inline(Some("sidebar"))],
        };

        run(&spec, &session_id, &store);

        let surfaces = store.list_resumable_surfaces().unwrap();
        assert!(surfaces
            .iter()
            .any(|s| s.placement.as_deref() == Some("sidebar")));
    }

    #[test]
    fn null_placement_falls_back_to_default() {
        let store = make_store();
        let session_id = make_session(&store);
        let spec = LaunchSpec {
            version: 1,
            items: vec![item_with_inline(None)],
        };

        run(&spec, &session_id, &store);

        let surfaces = store.list_resumable_surfaces().unwrap();
        assert!(surfaces.iter().any(|s| s.placement.is_none()));
    }
}
