use crate::context::Ctx;
use crate::shared::cqs::Command;
use crate::shared::Result;

/// Re-read all user-config from disk (pick up external edits). The stores have
/// no in-memory cache so subsequent reads already reflect disk state; this
/// command exists as an explicit reload signal and is a no-op structurally.
pub struct ReloadConfig;

impl Command<Ctx> for ReloadConfig {
    async fn handle(&self, _cx: &Ctx) -> Result<()> {
        Ok(())
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
    async fn reload_config_returns_ok_and_subsequent_reads_reflect_disk() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        bus.execute(ApplySetting {
            scope: SettingScope::Global,
            key: "k".to_owned(),
            value_json: r#""old""#.to_owned(),
        })
        .await
        .unwrap();

        // Simulate external edit.
        let path = dir
            .path()
            .join("config")
            .join("settings")
            .join("global.json");
        let raw = std::fs::read_to_string(&path).unwrap();
        let mut map: std::collections::HashMap<String, String> =
            serde_json::from_str(&raw).unwrap();
        map.insert("k".to_owned(), r#""new""#.to_owned());
        std::fs::write(&path, serde_json::to_string_pretty(&map).unwrap()).unwrap();

        bus.execute(ReloadConfig).await.unwrap();

        let v = bus
            .query(GetSetting {
                scope: SettingScope::Global,
                key: "k".to_owned(),
            })
            .await
            .unwrap();
        assert_eq!(v.as_deref(), Some(r#""new""#));
    }
}
