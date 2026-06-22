use crate::context::Ctx;
use crate::infra::config::profile::Profile;
use crate::infra::config::ProfileStore;
use crate::shared::cqs::Query;
use crate::shared::Result;

/// The currently active profile. Returns `None` if not set.
pub struct GetActiveProfile;

impl Query<Ctx> for GetActiveProfile {
    type Out = Option<Profile>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        ProfileStore::new(cx.fs_root()).get_active().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::settings::test_util::*;
    use crate::shared::bus::Bus;

    #[tokio::test]
    async fn get_active_profile_returns_none_before_activation() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        let active = bus.query(GetActiveProfile).await.unwrap();
        assert!(active.is_none());
    }
}
