use serde::Deserialize;

use crate::context::Ctx;
use crate::infra::config::ThemeStore;
use crate::shared::message::Query;
use crate::shared::Result;

/// Export a theme bundle as bytes. Returns `None` if not found.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportTheme {
    pub id: String,
}

impl Query<Ctx> for ExportTheme {
    type Out = Option<Vec<u8>>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        ThemeStore::new(cx.fs_root()).export(&self.id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::settings::import_theme::ImportTheme;
    use crate::app::settings::test_util::*;
    use crate::infra::config::theme::Theme;
    use crate::shared::bus::Bus;

    #[tokio::test]
    async fn export_theme_round_trips() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        bus.execute(ImportTheme {
            id: "t1".to_owned(),
            name: "T1".to_owned(),
            origin: "custom".to_owned(),
            data_json: Some(r#"{"color":"red"}"#.to_owned()),
        })
        .await
        .unwrap();

        let bytes = bus
            .query(ExportTheme {
                id: "t1".to_owned(),
            })
            .await
            .unwrap()
            .unwrap();

        let t: Theme = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(t.id, "t1");
        assert_eq!(t.data_json.as_deref(), Some(r#"{"color":"red"}"#));
    }
}
