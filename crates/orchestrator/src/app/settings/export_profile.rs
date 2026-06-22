use serde::Deserialize;

use crate::context::Ctx;
use crate::infra::config::ProfileStore;
use crate::shared::message::Query;
use crate::shared::Result;

/// Export a profile as its JSON bundle bytes. Returns `None` if not found.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportProfile {
    pub id: String,
}

impl Query<Ctx> for ExportProfile {
    type Out = Option<Vec<u8>>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        let store = ProfileStore::new(cx.fs_root());
        match store.get(&self.id).await? {
            None => Ok(None),
            Some(profile) => {
                let bytes = serde_json::to_vec_pretty(&profile)?;
                Ok(Some(bytes))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::settings::test_util::*;
    use crate::shared::bus::Bus;

    #[tokio::test]
    async fn export_absent_profile_returns_none() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        let result = bus
            .query(ExportProfile {
                id: "nope".to_owned(),
            })
            .await
            .unwrap();
        assert!(result.is_none());
    }
}
