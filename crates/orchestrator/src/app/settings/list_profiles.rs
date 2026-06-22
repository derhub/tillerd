use crate::context::Ctx;
use crate::infra::config::profile::Profile;
use crate::infra::config::ProfileStore;
use crate::shared::cqs::Query;
use crate::shared::Result;

/// All profiles, sorted by id.
pub struct ListProfiles;

impl Query<Ctx> for ListProfiles {
    type Out = Vec<Profile>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        ProfileStore::new(cx.fs_root()).list().await
    }
}
