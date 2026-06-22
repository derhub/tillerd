use crate::context::Ctx;
use crate::entities::setting::{SettingEntry, SettingScope};
use crate::infra::config::SettingStore;
use crate::shared::cqs::Query;
use crate::shared::Result;

/// All overrides at a scope, sorted by key.
pub struct ListSettings {
    pub scope: SettingScope,
}

impl Query<Ctx> for ListSettings {
    type Out = Vec<SettingEntry>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        SettingStore::new(cx.fs_root()).list(&self.scope).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::settings::apply_setting::ApplySetting;
    use crate::app::settings::test_util::*;
    use crate::entities::setting::SettingScope;
    use crate::shared::bus::Bus;

    #[tokio::test]
    async fn list_settings_returns_all_overrides_sorted() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        bus.execute(ApplySetting {
            scope: SettingScope::Global,
            key: "z".to_owned(),
            value_json: "1".to_owned(),
        })
        .await
        .unwrap();
        bus.execute(ApplySetting {
            scope: SettingScope::Global,
            key: "a".to_owned(),
            value_json: "2".to_owned(),
        })
        .await
        .unwrap();

        let entries = bus
            .query(ListSettings {
                scope: SettingScope::Global,
            })
            .await
            .unwrap();

        let keys: Vec<&str> = entries.iter().map(|e| e.key.as_str()).collect();
        assert_eq!(keys, vec!["a", "z"]);
    }
}
