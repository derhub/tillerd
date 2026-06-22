use serde::Deserialize;

use crate::context::Ctx;
use crate::infra::config::ProfileStore;
use crate::shared::message::Command;
use crate::shared::Result;

/// Delete a profile.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscardProfile {
    pub id: String,
}

impl Command<Ctx> for DiscardProfile {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        ProfileStore::new(cx.fs_root()).delete(&self.id).await
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
    async fn discard_profile_removes_it() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        bus.execute(NewProfile {
            id: "p1".to_owned(),
            name: "P".to_owned(),
        })
        .await
        .unwrap();

        bus.execute(DiscardProfile {
            id: "p1".to_owned(),
        })
        .await
        .unwrap();

        let profiles = bus.query(ListProfiles).await.unwrap();
        assert!(profiles.is_empty());
    }
}
