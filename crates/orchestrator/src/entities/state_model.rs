//! Machine-readable state-model tables (ADR-0044): per-entity lifecycle states, legal
//! transitions, and guard rules, derived from the enums and `guard_*` methods in this
//! layer. Data only — no behavior. The committed `state-model.contract.json` fixture is
//! generated from these tables; a Rust test and a TS test assert their side against the
//! fixture, so drift on either side fails the build.

use serde::Serialize;

use super::project::{ProjectId, ProjectStatus};
use super::session::SessionStatus;
use super::surface::SurfaceStatus;
use super::workspace::{WorkspaceId, WorkspaceStatus};

/// A legal lifecycle transition, keyed by the operation that performs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Transition {
    pub action: &'static str,
    pub from: &'static str,
    pub to: &'static str,
}

/// A guard applied by an operation: the rule id and the entity fields a client
/// needs to evaluate it advisorily.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct GuardRule {
    pub action: &'static str,
    pub rule: &'static str,
    pub fields: &'static [&'static str],
}

/// One entity's state-model tables.
#[derive(Debug, Clone, Serialize)]
pub struct EntityStateModel {
    pub entity: &'static str,
    pub states: Vec<&'static str>,
    pub transitions: Vec<Transition>,
    pub guards: Vec<GuardRule>,
}

/// The full contract: every entity's tables plus the well-known ids guard rules
/// compare against.
#[derive(Debug, Clone, Serialize)]
pub struct StateModel {
    pub well_known_ids: WellKnownIds,
    pub entities: Vec<EntityStateModel>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WellKnownIds {
    pub default_workspace: &'static str,
    pub unfiled_project: &'static str,
}

/// Build the contract tables. State names come from the enums' `as_str` values so an
/// enum rename propagates here; actions and guard rules mirror the `guard_*` calls in
/// the app layer (the contract test keeps both sides honest).
pub fn state_model() -> StateModel {
    let container_states = |active: &'static str, archived: &'static str| vec![active, archived];

    StateModel {
        well_known_ids: WellKnownIds {
            default_workspace: WorkspaceId::DEFAULT,
            unfiled_project: ProjectId::UNFILED,
        },
        entities: vec![
            EntityStateModel {
                entity: "workspace",
                states: container_states(
                    WorkspaceStatus::Active.as_str(),
                    WorkspaceStatus::Archived.as_str(),
                ),
                transitions: vec![
                    Transition { action: "archive", from: "active", to: "archived" },
                    Transition { action: "restore", from: "archived", to: "active" },
                ],
                guards: vec![
                    GuardRule { action: "archive", rule: "not-default", fields: &["id"] },
                    GuardRule { action: "archive", rule: "active", fields: &["status"] },
                    GuardRule { action: "discard", rule: "not-default", fields: &["id"] },
                    GuardRule { action: "restore", rule: "archived", fields: &["status"] },
                ],
            },
            EntityStateModel {
                entity: "project",
                states: container_states(
                    ProjectStatus::Active.as_str(),
                    ProjectStatus::Archived.as_str(),
                ),
                transitions: vec![
                    Transition { action: "archive", from: "active", to: "archived" },
                    Transition { action: "restore", from: "archived", to: "active" },
                ],
                guards: vec![
                    GuardRule { action: "archive", rule: "not-unfiled", fields: &["id"] },
                    GuardRule { action: "archive", rule: "active", fields: &["status"] },
                    GuardRule { action: "discard", rule: "not-unfiled", fields: &["id"] },
                    GuardRule { action: "move", rule: "not-unfiled", fields: &["id"] },
                    GuardRule { action: "restore", rule: "archived", fields: &["status"] },
                ],
            },
            EntityStateModel {
                entity: "session",
                states: {
                    // SessionStatus has no as_str (never read as a string in Rust);
                    // name the sqlx snake_case wire values directly.
                    let _ = SessionStatus::Active;
                    vec!["active", "archived"]
                },
                transitions: vec![
                    Transition { action: "archive", from: "active", to: "archived" },
                    Transition { action: "restore", from: "archived", to: "active" },
                ],
                guards: vec![],
            },
            EntityStateModel {
                entity: "surface",
                states: vec![
                    SurfaceStatus::Pending.as_str(),
                    SurfaceStatus::Live.as_str(),
                    SurfaceStatus::Failed.as_str(),
                    SurfaceStatus::Idle.as_str(),
                ],
                transitions: vec![
                    Transition { action: "spawn", from: "pending", to: "live" },
                    Transition { action: "spawn", from: "pending", to: "failed" },
                    Transition { action: "resume", from: "idle", to: "live" },
                    Transition { action: "resume", from: "failed", to: "live" },
                    Transition { action: "stop", from: "live", to: "idle" },
                    Transition { action: "reconcile", from: "live", to: "failed" },
                    Transition { action: "reconcile", from: "pending", to: "failed" },
                ],
                guards: vec![],
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("state-model.contract.json");

    fn serialized() -> String {
        let mut out = serde_json::to_string_pretty(&state_model()).expect("state model serializes");
        out.push('\n');
        out
    }

    /// Contract: the committed fixture matches the tables. Regenerate deliberately with
    /// `TILLERD_BLESS=1 cargo nextest run -p orchestrator state_model` after a reviewed
    /// state-model change; the fixture diff is the reviewable artifact.
    #[test]
    fn committed_fixture_matches_the_state_model_tables() {
        let current = serialized();
        if std::env::var_os("TILLERD_BLESS").is_some() {
            let path = concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/entities/state-model.contract.json"
            );
            std::fs::write(path, &current).expect("bless writes the fixture");
        }
        assert_eq!(
            FIXTURE, current,
            "state-model.contract.json is stale; regenerate with TILLERD_BLESS=1 and review the diff"
        );
    }

    #[test]
    fn every_transition_endpoint_is_a_declared_state() {
        for entity in state_model().entities {
            for t in &entity.transitions {
                assert!(
                    entity.states.contains(&t.from) && entity.states.contains(&t.to),
                    "{}: transition {}:{}->{} references an undeclared state",
                    entity.entity,
                    t.action,
                    t.from,
                    t.to
                );
            }
        }
    }

    #[test]
    fn guard_rules_reference_well_known_ids_consistently() {
        let model = state_model();
        assert_eq!(model.well_known_ids.default_workspace, WorkspaceId::DEFAULT);
        assert_eq!(model.well_known_ids.unfiled_project, ProjectId::UNFILED);
    }
}
