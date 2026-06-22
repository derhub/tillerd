use serde::Deserialize;

use crate::app::settings::common::scope_from_parts;
use crate::context::Ctx;
use crate::infra::config::SettingStore;
use crate::shared::message::Command;
use crate::shared::Result;

/// Set or overwrite a setting value at a scope. `value_json` carries the JSON value
/// as a serialized string (the wire is a JSON value; it is persisted verbatim).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplySetting {
    pub scope: String,
    pub project_id: Option<String>,
    pub key: String,
    pub value_json: String,
}

impl Command<Ctx> for ApplySetting {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let scope = scope_from_parts(&self.scope, self.project_id.as_deref())?;
        SettingStore::new(cx.fs_root())
            .apply(&scope, &self.key, &self.value_json)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::settings::get_setting::GetSetting;
    use crate::app::settings::test_util::*;
    use crate::shared::bus::Bus;

    #[tokio::test]
    async fn apply_setting_then_get_returns_value() {
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

        let v = bus
            .query(GetSetting {
                scope: "global".to_owned(),
                project_id: None,
                key: "theme".to_owned(),
            })
            .await
            .unwrap();

        assert_eq!(v.as_deref(), Some(r#""dark""#));
    }
}
