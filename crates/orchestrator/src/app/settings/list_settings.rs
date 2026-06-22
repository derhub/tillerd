use serde::Deserialize;

use crate::app::settings::common::scope_from_parts;
use crate::app::settings::SettingView;
use crate::context::Ctx;
use crate::infra::config::SettingStore;
use crate::shared::message::Query;
use crate::shared::Result;

/// All overrides at a scope, sorted by key.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSettings {
    pub scope: String,
    pub project_id: Option<String>,
}

impl Query<Ctx> for ListSettings {
    type Out = Vec<SettingView>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        let scope = scope_from_parts(&self.scope, self.project_id.as_deref())?;
        let entries = SettingStore::new(cx.fs_root()).list(&scope).await?;
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
    async fn list_settings_returns_all_overrides_sorted() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        bus.execute(ApplySetting {
            scope: "global".to_owned(),
            project_id: None,
            key: "z".to_owned(),
            value_json: "1".to_owned(),
        })
        .await
        .unwrap();
        bus.execute(ApplySetting {
            scope: "global".to_owned(),
            project_id: None,
            key: "a".to_owned(),
            value_json: "2".to_owned(),
        })
        .await
        .unwrap();

        let entries = bus
            .query(ListSettings {
                scope: "global".to_owned(),
                project_id: None,
            })
            .await
            .unwrap();

        let keys: Vec<&str> = entries.iter().map(|e| e.key.as_str()).collect();
        assert_eq!(keys, vec!["a", "z"]);
    }
}
