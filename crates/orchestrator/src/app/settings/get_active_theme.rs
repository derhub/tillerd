use crate::context::Ctx;
use crate::infra::config::theme::Theme;
use crate::infra::config::ThemeStore;
use crate::shared::cqs::Query;
use crate::shared::Result;

/// The currently active theme. Returns `None` if not set.
pub struct GetActiveTheme;

impl Query<Ctx> for GetActiveTheme {
    type Out = Option<Theme>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        ThemeStore::new(cx.fs_root()).get_active().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::settings::test_util::*;
    use crate::shared::bus::Bus;

    #[tokio::test]
    async fn get_active_theme_returns_none_before_activation() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        assert!(bus.query(GetActiveTheme).await.unwrap().is_none());
    }
}
