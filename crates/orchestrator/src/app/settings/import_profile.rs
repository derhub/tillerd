use serde::Deserialize;

use crate::context::Ctx;
use crate::infra::config::profile::Profile;
use crate::infra::config::ProfileStore;
use crate::shared::message::Command;
use crate::shared::Result;

/// Import a profile from a serialized JSON bundle (the same format `ProfileStore` persists).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportProfile {
    pub profile_json: String,
}

impl Command<Ctx> for ImportProfile {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let profile: Profile = serde_json::from_str(&self.profile_json)?;
        ProfileStore::new(cx.fs_root()).save(&profile).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::settings::export_profile::ExportProfile;
    use crate::app::settings::test_util::*;
    use crate::infra::config::profile::Profile;
    use crate::shared::bus::Bus;

    #[tokio::test]
    async fn import_and_export_profile_round_trips() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        let original_json = r#"{"id":"imported","name":"Imported","settings":{"k":"v"}}"#;

        bus.execute(ImportProfile {
            profile_json: original_json.to_owned(),
        })
        .await
        .unwrap();

        let exported = bus
            .query(ExportProfile {
                id: "imported".to_owned(),
            })
            .await
            .unwrap()
            .unwrap();

        let loaded: Profile = serde_json::from_slice(&exported).unwrap();
        assert_eq!(loaded.id, "imported");
        assert_eq!(loaded.name, "Imported");
        assert_eq!(loaded.settings.get("k").map(String::as_str), Some("v"));
    }
}
