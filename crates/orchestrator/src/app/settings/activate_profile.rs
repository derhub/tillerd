use crate::context::Ctx;
use crate::infra::config::ProfileStore;
use crate::shared::cqs::Command;
use crate::shared::{Error, Result};

/// Switch the active profile.
pub struct ActivateProfile {
    pub id: String,
}

impl Command<Ctx> for ActivateProfile {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let store = ProfileStore::new(cx.fs_root());
        // Guard: reject activation if the profile does not exist.
        store
            .get(&self.id)
            .await?
            .ok_or_else(|| Error::ProfileNotFound(self.id.clone()))?;
        store.set_active(&self.id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::settings::get_active_profile::GetActiveProfile;
    use crate::app::settings::new_profile::NewProfile;
    use crate::app::settings::test_util::*;
    use crate::shared::bus::Bus;

    #[tokio::test]
    async fn activate_profile_changes_get_active() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        bus.execute(NewProfile {
            id: "p1".to_owned(),
            name: "One".to_owned(),
        })
        .await
        .unwrap();
        bus.execute(NewProfile {
            id: "p2".to_owned(),
            name: "Two".to_owned(),
        })
        .await
        .unwrap();

        bus.execute(ActivateProfile {
            id: "p1".to_owned(),
        })
        .await
        .unwrap();
        bus.execute(ActivateProfile {
            id: "p2".to_owned(),
        })
        .await
        .unwrap();

        let active = bus.query(GetActiveProfile).await.unwrap();
        assert_eq!(active.as_ref().map(|p| p.id.as_str()), Some("p2"));
    }

    #[tokio::test]
    async fn activate_absent_profile_returns_error() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        let err = bus
            .execute(ActivateProfile {
                id: "nope".to_owned(),
            })
            .await
            .unwrap_err();

        assert_eq!(err.code(), "profile.not_found");
    }
}
