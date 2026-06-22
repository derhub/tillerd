use std::collections::HashMap;

use crate::context::Ctx;
use crate::infra::config::KeybindingStore;
use crate::shared::cqs::Command;
use crate::shared::Result;

/// Revert one action to its compiled-in default.
pub struct ResetKeybinding {
    pub action: String,
    pub defaults: HashMap<String, String>,
}

impl Command<Ctx> for ResetKeybinding {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        KeybindingStore::new(cx.fs_root(), self.defaults.clone())
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
            defaults: default_keys(),
        })
        .await
        .unwrap();

        bus.execute(ResetKeybinding {
            action: "rename".to_owned(),
            defaults: default_keys(),
        })
        .await
        .unwrap();

        let chord = bus
            .query(ResolveKeybinding {
                action: "rename".to_owned(),
                defaults: default_keys(),
            })
            .await
            .unwrap();

        assert_eq!(chord.as_deref(), Some("F2"));
    }
}
