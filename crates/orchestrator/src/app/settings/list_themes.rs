use serde::Deserialize;

use crate::app::settings::ThemeView;
use crate::context::Ctx;
use crate::infra::config::ThemeStore;
use crate::shared::message::Query;
use crate::shared::Result;

/// All themes (prebuilt + custom), sorted by id.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListThemes;

impl Query<Ctx> for ListThemes {
    type Out = Vec<ThemeView>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        Ok(ThemeStore::new(cx.fs_root())
            .list()
            .await?
            .into_iter()
            .map(ThemeView)
            .collect())
    }
}
