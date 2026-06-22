use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::project::ProjectId;
use crate::infra::config::SettingStore;
use crate::shared::message::Query;
use crate::shared::Result;

/// Effective value for a project after the profile cascade: project override if
/// present, else global, else `None`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveSetting {
    pub project_id: String,
    pub key: String,
}

impl Query<Ctx> for ResolveSetting {
    type Out = Option<String>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        let project_id = ProjectId::new(&self.project_id);
        SettingStore::new(cx.fs_root())
            .resolve(&project_id, &self.key)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::settings::apply_setting::ApplySetting;
    use crate::app::settings::test_util::*;
    use crate::shared::bus::Bus;

    #[tokio::test]
    async fn resolve_setting_returns_project_override_over_global() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        bus.execute(ApplySetting {
            scope: "global".to_owned(),
            project_id: None,
            key: "env".to_owned(),
            value_json: r#""global""#.to_owned(),
        })
        .await
        .unwrap();
        bus.execute(ApplySetting {
            scope: "project".to_owned(),
            project_id: Some("proj-1".to_owned()),
            key: "env".to_owned(),
            value_json: r#""project""#.to_owned(),
        })
        .await
        .unwrap();

        let v = bus
            .query(ResolveSetting {
                project_id: "proj-1".to_owned(),
                key: "env".to_owned(),
            })
            .await
            .unwrap();

        assert_eq!(v.as_deref(), Some(r#""project""#));
    }
}
