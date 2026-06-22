use serde::Deserialize;

use crate::app::settings::common::scope_from_parts;
use crate::context::Ctx;
use crate::infra::config::SettingStore;
use crate::shared::message::Query;
use crate::shared::Result;

/// Raw setting value at a scope. Returns `None` if absent.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSetting {
    pub scope: String,
    pub project_id: Option<String>,
    pub key: String,
}

impl Query<Ctx> for GetSetting {
    type Out = Option<String>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        let scope = scope_from_parts(&self.scope, self.project_id.as_deref())?;
        SettingStore::new(cx.fs_root()).get(&scope, &self.key).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::settings::test_util::*;
    use crate::shared::bus::Bus;

    #[tokio::test]
    async fn get_setting_returns_none_for_absent_key() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        let v = bus
            .query(GetSetting {
                scope: "global".to_owned(),
                project_id: None,
                key: "missing".to_owned(),
            })
            .await
            .unwrap();

        assert_eq!(v, None);
    }
}
