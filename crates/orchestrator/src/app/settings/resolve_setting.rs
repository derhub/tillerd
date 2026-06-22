use crate::context::Ctx;
use crate::entities::project::ProjectId;
use crate::infra::config::SettingStore;
use crate::shared::cqs::Query;
use crate::shared::Result;

/// Effective value for a project after the profile cascade: project override if
/// present, else global, else `None`.
pub struct ResolveSetting {
    pub project_id: ProjectId,
    pub key: String,
}

impl Query<Ctx> for ResolveSetting {
    type Out = Option<String>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        SettingStore::new(cx.fs_root())
            .resolve(&self.project_id, &self.key)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::settings::apply_setting::ApplySetting;
    use crate::app::settings::test_util::*;
    use crate::entities::project::ProjectId;
    use crate::entities::setting::SettingScope;
    use crate::shared::bus::Bus;

    #[tokio::test]
    async fn resolve_setting_returns_project_override_over_global() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);
        let pid = ProjectId::new("proj-1".to_owned());

        bus.execute(ApplySetting {
            scope: SettingScope::Global,
            key: "env".to_owned(),
            value_json: r#""global""#.to_owned(),
        })
        .await
        .unwrap();
        bus.execute(ApplySetting {
            scope: SettingScope::Project(pid.clone()),
            key: "env".to_owned(),
            value_json: r#""project""#.to_owned(),
        })
        .await
        .unwrap();

        let v = bus
            .query(ResolveSetting {
                project_id: pid.clone(),
                key: "env".to_owned(),
            })
            .await
            .unwrap();

        assert_eq!(v.as_deref(), Some(r#""project""#));
    }
}
