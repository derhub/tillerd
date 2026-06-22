use serde::Deserialize;

use crate::context::Ctx;
use crate::infra::config::ProfileStore;
use crate::shared::message::Command;
use crate::shared::Result;

/// Create a new profile.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewProfile {
    pub id: String,
    pub name: String,
}

impl Command<Ctx> for NewProfile {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        ProfileStore::new(cx.fs_root())
            .create(&self.id, &self.name)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::settings::list_profiles::ListProfiles;
    use crate::app::settings::test_util::*;
    use crate::shared::bus::Bus;

    #[tokio::test]
    async fn new_profile_then_list_includes_it() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        bus.execute(NewProfile {
            id: "p1".to_owned(),
            name: "My Profile".to_owned(),
        })
        .await
        .unwrap();

        let profiles = bus.query(ListProfiles).await.unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].0.name, "My Profile");
    }
}
