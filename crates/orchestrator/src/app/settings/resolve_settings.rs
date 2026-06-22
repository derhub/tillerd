use crate::context::Ctx;
use crate::entities::project::ProjectId;
use crate::entities::setting::SettingEntry;
use crate::infra::config::SettingStore;
use crate::shared::cqs::Query;
use crate::shared::Result;

/// Full effective settings map for a project: global defaults merged with
/// project-scoped overrides (project wins on collision), sorted by key.
pub struct ResolveSettings {
    pub project_id: ProjectId,
}

impl Query<Ctx> for ResolveSettings {
    type Out = Vec<SettingEntry>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        SettingStore::new(cx.fs_root())
            .resolve_all(&self.project_id)
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
    async fn resolve_settings_merges_global_and_project() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);
        let pid = ProjectId::new("proj-1".to_owned());

        bus.execute(ApplySetting {
            scope: SettingScope::Global,
            key: "a".to_owned(),
            value_json: r#""ga""#.to_owned(),
        })
        .await
        .unwrap();
        bus.execute(ApplySetting {
            scope: SettingScope::Project(pid.clone()),
            key: "b".to_owned(),
            value_json: r#""pb""#.to_owned(),
        })
        .await
        .unwrap();

        let entries = bus
            .query(ResolveSettings {
                project_id: pid.clone(),
            })
            .await
            .unwrap();

        let map: std::collections::HashMap<_, _> = entries
            .iter()
            .map(|e| (e.key.as_str(), e.value_json.as_str()))
            .collect();
        assert_eq!(map["a"], r#""ga""#);
        assert_eq!(map["b"], r#""pb""#);
    }
}
