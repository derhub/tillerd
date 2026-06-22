use std::collections::HashMap;

use serde::Deserialize;

use crate::context::Ctx;
use crate::infra::config::KeybindingStore;
use crate::shared::message::Command;
use crate::shared::Result;

/// Set or override an action's chord. Carries the defaults (as a serialized
/// `{action: chord}` map) so the store can merge effectively.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RebindKey {
    pub action: String,
    pub chord: String,
    pub defaults_json: String,
}

impl Command<Ctx> for RebindKey {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let defaults: HashMap<String, String> = serde_json::from_str(&self.defaults_json)?;
        KeybindingStore::new(cx.fs_root(), defaults)
            .rebind(&self.action, &self.chord)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::settings::list_keybindings::ListKeybindings;
    use crate::app::settings::test_util::*;
    use crate::shared::bus::Bus;

    #[tokio::test]
    async fn rebind_key_then_list_reflects_override() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        bus.execute(RebindKey {
            action: "rename".to_owned(),
            chord: "ctrl+r".to_owned(),
            defaults_json: default_keys_json(),
        })
        .await
        .unwrap();

        let entries = bus
            .query(ListKeybindings {
                defaults_json: default_keys_json(),
            })
            .await
            .unwrap();

        let rename = entries.iter().find(|e| e.0.action == "rename").unwrap();
        assert_eq!(rename.0.chord, "ctrl+r");
    }
}
