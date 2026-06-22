use serde::Deserialize;

use crate::app::settings::ProfileView;
use crate::context::Ctx;
use crate::infra::config::ProfileStore;
use crate::shared::message::Query;
use crate::shared::Result;

/// All profiles, sorted by id.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListProfiles;

impl Query<Ctx> for ListProfiles {
    type Out = Vec<ProfileView>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        Ok(ProfileStore::new(cx.fs_root())
            .list()
            .await?
            .into_iter()
            .map(ProfileView)
            .collect())
    }
}
