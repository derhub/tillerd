use serde::Deserialize;

use crate::context::Ctx;
use crate::infra::config::ProfileStore;
use crate::shared::message::Command;
use crate::shared::{Error, Result};

/// Copy a profile under a new id and name. The copy is independent.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateProfile {
    pub source_id: String,
    pub new_id: String,
    pub new_name: String,
}

impl Command<Ctx> for DuplicateProfile {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let store = ProfileStore::new(cx.fs_root());
        store
            .duplicate(&self.source_id, &self.new_id, &self.new_name)
            .await?
            .ok_or_else(|| Error::ProfileNotFound(self.source_id.clone()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::settings::list_profiles::ListProfiles;
    use crate::app::settings::new_profile::NewProfile;
    use crate::app::settings::rename_profile::RenameProfile;
    use crate::app::settings::test_util::*;
    use crate::shared::bus::Bus;

    #[tokio::test]
    async fn duplicate_profile_creates_independent_copy() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        bus.execute(NewProfile {
            id: "orig".to_owned(),
            name: "Original".to_owned(),
        })
        .await
        .unwrap();

        bus.execute(DuplicateProfile {
            source_id: "orig".to_owned(),
            new_id: "copy".to_owned(),
            new_name: "Copy".to_owned(),
        })
        .await
        .unwrap();

        let profiles = bus.query(ListProfiles).await.unwrap();
        assert_eq!(profiles.len(), 2);

        // Copy is independent: renaming original does not affect it.
        bus.execute(RenameProfile {
            id: "orig".to_owned(),
            new_name: "Changed".to_owned(),
        })
        .await
        .unwrap();

        let profiles = bus.query(ListProfiles).await.unwrap();
        let copy = profiles.iter().find(|p| p.0.id == "copy").unwrap();
        assert_eq!(copy.0.name, "Copy");
    }
}
