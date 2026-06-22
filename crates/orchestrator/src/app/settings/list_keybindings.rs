use std::collections::HashMap;

use serde::Deserialize;

use crate::app::settings::KeybindingView;
use crate::context::Ctx;
use crate::infra::config::KeybindingStore;
use crate::shared::message::Query;
use crate::shared::Result;

/// The effective keymap: defaults merged with user overrides. `defaults_json`
/// carries the compiled-in default keymap as a serialized `{action: chord}` map.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListKeybindings {
    pub defaults_json: String,
}

impl Query<Ctx> for ListKeybindings {
    type Out = Vec<KeybindingView>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        let defaults: HashMap<String, String> = serde_json::from_str(&self.defaults_json)?;
        Ok(KeybindingStore::new(cx.fs_root(), defaults)
            .list()
            .await?
            .into_iter()
            .map(KeybindingView)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::settings::test_util::*;
    use crate::shared::bus::Bus;

    #[tokio::test]
    async fn list_keybindings_includes_defaults() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        let entries = bus
            .query(ListKeybindings {
                defaults_json: default_keys_json(),
            })
            .await
            .unwrap();

        let new_sess = entries
            .iter()
            .find(|e| e.0.action == "new-session")
            .unwrap();
        assert_eq!(new_sess.0.chord, "ctrl+n");
    }
}
