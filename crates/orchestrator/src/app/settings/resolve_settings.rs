use std::collections::HashMap;

use serde::Deserialize;

use crate::app::settings::SettingView;
use crate::context::Ctx;
use crate::entities::project::ProjectId;
use crate::entities::setting::SettingScope;
use crate::infra::config::SettingStore;
use crate::shared::message::Query;
use crate::shared::Result;

/// Full effective settings map for a project: global defaults merged with
/// project-scoped overrides (project wins on collision), sorted by key.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveSettings {
    pub project_id: String,
}

impl Query<Ctx> for ResolveSettings {
    type Out = Vec<SettingView>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        let project_id = ProjectId::new(&self.project_id);
        let store = SettingStore::new(cx.fs_root());

        // Load both scopes as raw maps, project wins on collision.
        let global = store.list(&SettingScope::Global).await?;
        let project = store.list(&SettingScope::Project(project_id)).await?;

        let mut merged: HashMap<String, String> =
            global.into_iter().map(|e| (e.key, e.value_json)).collect();
        for e in project {
            merged.insert(e.key, e.value_json);
        }

        let mut entries: Vec<SettingView> = merged
            .into_iter()
            .map(|(key, value_json)| {
                Ok(SettingView {
                    key,
                    value: serde_json::from_str(&value_json)?,
                })
            })
            .collect::<Result<_>>()?;
        entries.sort_by(|a, b| a.key.cmp(&b.key));
        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::settings::apply_setting::ApplySetting;
    use crate::app::settings::test_util::*;
    use crate::shared::bus::Bus;

    // Scenario: global and project entries both appear; project key is present
    #[tokio::test]
    async fn resolve_settings_merges_global_and_project() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        bus.execute(ApplySetting {
            scope: "global".to_owned(),
            project_id: None,
            key: "a".to_owned(),
            value_json: r#""ga""#.to_owned(),
        })
        .await
        .unwrap();
        bus.execute(ApplySetting {
            scope: "project".to_owned(),
            project_id: Some("proj-1".to_owned()),
            key: "b".to_owned(),
            value_json: r#""pb""#.to_owned(),
        })
        .await
        .unwrap();

        let entries = bus
            .query(ResolveSettings {
                project_id: "proj-1".to_owned(),
            })
            .await
            .unwrap();

        let map: std::collections::HashMap<_, _> =
            entries.iter().map(|e| (e.key.as_str(), &e.value)).collect();
        assert_eq!(map["a"], &serde_json::json!("ga"));
        assert_eq!(map["b"], &serde_json::json!("pb"));
    }

    // Scenario: project value overrides global for the same key
    #[tokio::test]
    async fn resolve_settings_project_wins_on_collision() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        bus.execute(ApplySetting {
            scope: "global".to_owned(),
            project_id: None,
            key: "theme".to_owned(),
            value_json: r#""dark""#.to_owned(),
        })
        .await
        .unwrap();
        bus.execute(ApplySetting {
            scope: "project".to_owned(),
            project_id: Some("proj-2".to_owned()),
            key: "theme".to_owned(),
            value_json: r#""light""#.to_owned(),
        })
        .await
        .unwrap();

        let entries = bus
            .query(ResolveSettings {
                project_id: "proj-2".to_owned(),
            })
            .await
            .unwrap();

        let theme = entries.iter().find(|e| e.key == "theme").unwrap();
        assert_eq!(theme.value, serde_json::json!("light"));
    }

    // Scenario: result is sorted by key
    #[tokio::test]
    async fn resolve_settings_result_is_sorted_by_key() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        for key in ["z-key", "a-key", "m-key"] {
            bus.execute(ApplySetting {
                scope: "global".to_owned(),
                project_id: None,
                key: key.to_owned(),
                value_json: "1".to_owned(),
            })
            .await
            .unwrap();
        }

        let entries = bus
            .query(ResolveSettings {
                project_id: "proj-1".to_owned(),
            })
            .await
            .unwrap();

        let keys: Vec<&str> = entries.iter().map(|e| e.key.as_str()).collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(keys, sorted);
    }
}
