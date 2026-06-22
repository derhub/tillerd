use crate::context::Ctx;
use crate::entities::setting::SettingScope;
use crate::infra::config::SettingStore;
use crate::shared::cqs::Command;
use crate::shared::Result;

/// Set or overwrite a setting value at a scope.
pub struct ApplySetting {
    pub scope: SettingScope,
    pub key: String,
    pub value_json: String,
}

impl Command<Ctx> for ApplySetting {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        SettingStore::new(cx.fs_root())
            .apply(&self.scope, &self.key, &self.value_json)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::settings::get_setting::GetSetting;
    use crate::app::settings::test_util::*;
    use crate::entities::setting::SettingScope;
    use crate::shared::bus::Bus;

    #[tokio::test]
    async fn apply_setting_then_get_returns_value() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        bus.execute(ApplySetting {
            scope: SettingScope::Global,
            key: "theme".to_owned(),
            value_json: r#""dark""#.to_owned(),
        })
        .await
        .unwrap();

        let v = bus
            .query(GetSetting {
                scope: SettingScope::Global,
                key: "theme".to_owned(),
            })
            .await
            .unwrap();

        assert_eq!(v.as_deref(), Some(r#""dark""#));
    }
}
