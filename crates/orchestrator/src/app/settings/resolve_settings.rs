use serde::Deserialize;

use crate::app::settings::SettingView;
use crate::context::Ctx;
use crate::entities::project::ProjectId;
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
        let entries = SettingStore::new(cx.fs_root())
            .resolve_all(&project_id)
            .await?;
        entries
            .into_iter()
            .map(|e| {
                Ok(SettingView {
                    key: e.key,
                    value: serde_json::from_str(&e.value_json)?,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::settings::apply_setting::ApplySetting;
    use crate::app::settings::test_util::*;
    use crate::shared::bus::Bus;

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
}
