use crate::context::Ctx;
use crate::entities::setting::SettingScope;
use crate::infra::config::SettingStore;
use crate::shared::cqs::Query;
use crate::shared::Result;

/// Raw setting value at a scope. Returns `None` if absent.
pub struct GetSetting {
    pub scope: SettingScope,
    pub key: String,
}

impl Query<Ctx> for GetSetting {
    type Out = Option<String>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        SettingStore::new(cx.fs_root())
            .get(&self.scope, &self.key)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::settings::test_util::*;
    use crate::entities::setting::SettingScope;
    use crate::shared::bus::Bus;

    #[tokio::test]
    async fn get_setting_returns_none_for_absent_key() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        let v = bus
            .query(GetSetting {
                scope: SettingScope::Global,
                key: "missing".to_owned(),
            })
            .await
            .unwrap();

        assert_eq!(v, None);
    }
}
