use crate::context::Ctx;
use crate::infra::config::ThemeStore;
use crate::shared::cqs::Command;
use crate::shared::Result;

/// Remove a custom theme. Prebuilt themes are rejected.
pub struct DiscardTheme {
    pub id: String,
}

impl Command<Ctx> for DiscardTheme {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        ThemeStore::new(cx.fs_root()).discard(&self.id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::settings::import_theme::ImportTheme;
    use crate::app::settings::list_themes::ListThemes;
    use crate::app::settings::test_util::*;
    use crate::infra::config::theme::{Theme, ThemeOrigin};
    use crate::shared::bus::Bus;

    #[tokio::test]
    async fn discard_custom_theme_removes_it() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        bus.execute(ImportTheme {
            theme: Theme {
                id: "my-theme".to_owned(),
                name: "Mine".to_owned(),
                origin: ThemeOrigin::Custom,
                data_json: None,
            },
        })
        .await
        .unwrap();

        bus.execute(DiscardTheme {
            id: "my-theme".to_owned(),
        })
        .await
        .unwrap();

        let themes = bus.query(ListThemes).await.unwrap();
        assert!(themes.is_empty());
    }

    #[tokio::test]
    async fn discard_prebuilt_theme_returns_error() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        bus.execute(ImportTheme {
            theme: Theme {
                id: "builtin".to_owned(),
                name: "Builtin".to_owned(),
                origin: ThemeOrigin::Prebuilt,
                data_json: None,
            },
        })
        .await
        .unwrap();

        let err = bus
            .execute(DiscardTheme {
                id: "builtin".to_owned(),
            })
            .await
            .unwrap_err();

        assert_eq!(err.code(), "prebuilt.immutable");
    }
}
