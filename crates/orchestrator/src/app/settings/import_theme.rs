use crate::context::Ctx;
use crate::infra::config::theme::Theme;
use crate::infra::config::ThemeStore;
use crate::shared::cqs::Command;
use crate::shared::Result;

/// Register a theme (prebuilt or custom).
pub struct ImportTheme {
    pub theme: Theme,
}

impl Command<Ctx> for ImportTheme {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        ThemeStore::new(cx.fs_root()).import(&self.theme).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::settings::list_themes::ListThemes;
    use crate::app::settings::test_util::*;
    use crate::infra::config::theme::{Theme, ThemeOrigin};
    use crate::shared::bus::Bus;

    #[tokio::test]
    async fn import_theme_then_list_includes_it() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        bus.execute(ImportTheme {
            theme: Theme {
                id: "dark".to_owned(),
                name: "Dark".to_owned(),
                origin: ThemeOrigin::Custom,
                data_json: None,
            },
        })
        .await
        .unwrap();

        let themes = bus.query(ListThemes).await.unwrap();
        assert_eq!(themes.len(), 1);
        assert_eq!(themes[0].id, "dark");
    }
}
