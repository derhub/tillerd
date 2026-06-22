use crate::context::Ctx;
use crate::entities::setting::SettingScope;
use crate::infra::config::SettingStore;
use crate::shared::cqs::Command;
use crate::shared::Result;

/// Clear a setting override at a scope (revert to inherited/default).
pub struct ResetSetting {
    pub scope: SettingScope,
    pub key: String,
}

impl Command<Ctx> for ResetSetting {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        SettingStore::new(cx.fs_root())
            .reset(&self.scope, &self.key)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::settings::apply_setting::ApplySetting;
    use crate::app::settings::get_setting::GetSetting;
    use crate::app::settings::test_util::*;
    use crate::entities::setting::SettingScope;
    use crate::shared::bus::Bus;

    #[tokio::test]
    async fn reset_setting_removes_override() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        bus.execute(ApplySetting {
            scope: SettingScope::Global,
            key: "k".to_owned(),
            value_json: r#""v""#.to_owned(),
        })
        .await
        .unwrap();

        bus.execute(ResetSetting {
            scope: SettingScope::Global,
            key: "k".to_owned(),
        })
        .await
        .unwrap();

        let v = bus
            .query(GetSetting {
                scope: SettingScope::Global,
                key: "k".to_owned(),
            })
            .await
            .unwrap();
        assert_eq!(v, None);
    }
}
