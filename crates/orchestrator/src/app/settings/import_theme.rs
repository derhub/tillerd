use serde::Deserialize;

use crate::context::Ctx;
use crate::infra::config::theme::{Theme, ThemeOrigin};
use crate::infra::config::ThemeStore;
use crate::shared::message::Command;
use crate::shared::{Error, Result};

/// Register a theme (prebuilt or custom). `origin` is the wire discriminant
/// (`"prebuilt"` / `"custom"`); `data_json` is the opaque theme payload.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportTheme {
    pub id: String,
    pub name: String,
    pub origin: String,
    pub data_json: Option<String>,
}

/// Map the wire origin discriminant onto the [`ThemeOrigin`] value object.
pub(crate) fn origin_from_str(origin: &str) -> Result<ThemeOrigin> {
    match origin {
        "prebuilt" => Ok(ThemeOrigin::Prebuilt),
        "custom" => Ok(ThemeOrigin::Custom),
        other => Err(Error::Validation {
            field: "origin",
            reason: format!("unknown theme origin: {other}"),
        }),
    }
}

impl Command<Ctx> for ImportTheme {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let theme = Theme {
            id: self.id.clone(),
            name: self.name.clone(),
            origin: origin_from_str(&self.origin)?,
            data_json: self.data_json.clone(),
        };
        ThemeStore::new(cx.fs_root()).import(&theme).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::settings::list_themes::ListThemes;
    use crate::app::settings::test_util::*;
    use crate::shared::bus::Bus;

    #[tokio::test]
    async fn import_theme_then_list_includes_it() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        bus.execute(ImportTheme {
            id: "dark".to_owned(),
            name: "Dark".to_owned(),
            origin: "custom".to_owned(),
            data_json: None,
        })
        .await
        .unwrap();

        let themes = bus.query(ListThemes).await.unwrap();
        assert_eq!(themes.len(), 1);
        assert_eq!(themes[0].0.id, "dark");
    }
}
