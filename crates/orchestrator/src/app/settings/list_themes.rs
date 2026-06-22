use crate::context::Ctx;
use crate::infra::config::theme::Theme;
use crate::infra::config::ThemeStore;
use crate::shared::cqs::Query;
use crate::shared::Result;

/// All themes (prebuilt + custom), sorted by id.
pub struct ListThemes;

impl Query<Ctx> for ListThemes {
    type Out = Vec<Theme>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        ThemeStore::new(cx.fs_root()).list().await
    }
}
