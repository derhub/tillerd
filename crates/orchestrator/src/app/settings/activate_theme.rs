use crate::context::Ctx;
use crate::infra::config::ThemeStore;
use crate::shared::cqs::Command;
use crate::shared::{Error, Result};

/// Set the active theme.
pub struct ActivateTheme {
    pub id: String,
}

impl Command<Ctx> for ActivateTheme {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let store = ThemeStore::new(cx.fs_root());
        store
            .get(&self.id)
            .await?
            .ok_or_else(|| Error::ThemeNotFound(self.id.clone()))?;
        store.set_active(&self.id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::settings::get_active_theme::GetActiveTheme;
    use crate::app::settings::import_theme::ImportTheme;
    use crate::app::settings::test_util::*;
    use crate::infra::config::theme::{Theme, ThemeOrigin};
    use crate::shared::bus::Bus;

    #[tokio::test]
    async fn activate_theme_changes_get_active() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        bus.execute(ImportTheme {
            theme: Theme {
                id: "light".to_owned(),
                name: "Light".to_owned(),
                origin: ThemeOrigin::Prebuilt,
                data_json: None,
            },
        })
        .await
        .unwrap();

        bus.execute(ActivateTheme {
            id: "light".to_owned(),
        })
        .await
        .unwrap();

        let active = bus.query(GetActiveTheme).await.unwrap();
        assert_eq!(active.as_ref().map(|t| t.id.as_str()), Some("light"));
    }

    #[tokio::test]
    async fn activate_absent_theme_returns_error() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        let err = bus
            .execute(ActivateTheme {
                id: "nope".to_owned(),
            })
            .await
            .unwrap_err();

        assert_eq!(err.code(), "theme.not_found");
    }
}
