use serde::Deserialize;

use crate::context::Ctx;
use crate::infra::config::ProfileStore;
use crate::shared::message::Command;
use crate::shared::{Error, Result};

/// Rename an existing profile (updates its `name` field).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameProfile {
    pub id: String,
    pub new_name: String,
}

impl Command<Ctx> for RenameProfile {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let store = ProfileStore::new(cx.fs_root());
        let mut profile = store
            .get(&self.id)
            .await?
            .ok_or_else(|| Error::ProfileNotFound(self.id.clone()))?;
        profile.name = self.new_name.clone();
        store.save(&profile).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::settings::list_profiles::ListProfiles;
    use crate::app::settings::new_profile::NewProfile;
    use crate::app::settings::test_util::*;
    use crate::shared::bus::Bus;

    #[tokio::test]
    async fn rename_profile_updates_name() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        bus.execute(NewProfile {
            id: "p1".to_owned(),
            name: "Old".to_owned(),
        })
        .await
        .unwrap();

        bus.execute(RenameProfile {
            id: "p1".to_owned(),
            new_name: "New".to_owned(),
        })
        .await
        .unwrap();

        let profiles = bus.query(ListProfiles).await.unwrap();
        assert_eq!(profiles[0].0.name, "New");
    }

    #[tokio::test]
    async fn rename_absent_profile_returns_error() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        let err = bus
            .execute(RenameProfile {
                id: "nope".to_owned(),
                new_name: "X".to_owned(),
            })
            .await
            .unwrap_err();

        assert_eq!(err.code(), "profile.not_found");
    }
}
