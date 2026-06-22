use std::collections::HashMap;

use serde::Deserialize;

use crate::context::Ctx;
use crate::infra::config::KeybindingStore;
use crate::shared::message::Command;
use crate::shared::Result;

/// Revert one action to its compiled-in default. `defaults_json` carries the
/// default keymap as a serialized `{action: chord}` map.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetKeybinding {
    pub action: String,
    pub defaults_json: String,
}

impl Command<Ctx> for ResetKeybinding {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let defaults: HashMap<String, String> = serde_json::from_str(&self.defaults_json)?;
        KeybindingStore::new(cx.fs_root(), defaults)
            .reset(&self.action)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::settings::rebind_key::RebindKey;
    use crate::app::settings::resolve_keybinding::ResolveKeybinding;
    use crate::app::settings::test_util::*;
    use crate::shared::bus::Bus;

    #[tokio::test]
    async fn reset_keybinding_reverts_to_default() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        bus.execute(RebindKey {
            action: "rename".to_owned(),
            chord: "ctrl+r".to_owned(),
            defaults_json: default_keys_json(),
        })
        .await
        .unwrap();

        bus.execute(ResetKeybinding {
            action: "rename".to_owned(),
            defaults_json: default_keys_json(),
        })
        .await
        .unwrap();

        let chord = bus
            .query(ResolveKeybinding {
                action: "rename".to_owned(),
                defaults_json: default_keys_json(),
            })
            .await
            .unwrap();

        assert_eq!(chord.as_deref(), Some("F2"));
    }
}
