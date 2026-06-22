use serde::Deserialize;

use crate::app::settings::common::scope_from_parts;
use crate::context::Ctx;
use crate::infra::config::SettingStore;
use crate::shared::message::Command;
use crate::shared::Result;

/// Clear a setting override at a scope (revert to inherited/default).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetSetting {
    pub scope: String,
    pub project_id: Option<String>,
    pub key: String,
}

impl Command<Ctx> for ResetSetting {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let scope = scope_from_parts(&self.scope, self.project_id.as_deref())?;
        SettingStore::new(cx.fs_root())
            .reset(&scope, &self.key)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::settings::apply_setting::ApplySetting;
    use crate::app::settings::get_setting::GetSetting;
    use crate::app::settings::test_util::*;
    use crate::shared::bus::Bus;

    #[tokio::test]
    async fn reset_setting_removes_override() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        bus.execute(ApplySetting {
            scope: "global".to_owned(),
            project_id: None,
            key: "k".to_owned(),
            value_json: r#""v""#.to_owned(),
        })
        .await
        .unwrap();

        bus.execute(ResetSetting {
            scope: "global".to_owned(),
            project_id: None,
            key: "k".to_owned(),
        })
        .await
        .unwrap();

        let v = bus
            .query(GetSetting {
                scope: "global".to_owned(),
                project_id: None,
                key: "k".to_owned(),
            })
            .await
            .unwrap();
        assert_eq!(v, None);
    }
}
