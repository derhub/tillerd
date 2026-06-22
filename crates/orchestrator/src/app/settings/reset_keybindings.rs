use std::collections::HashMap;

use crate::context::Ctx;
use crate::infra::config::KeybindingStore;
use crate::shared::cqs::Command;
use crate::shared::Result;

/// Revert the entire keymap to defaults.
pub struct ResetKeybindings {
    pub defaults: HashMap<String, String>,
}

impl Command<Ctx> for ResetKeybindings {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        KeybindingStore::new(cx.fs_root(), self.defaults.clone())
            .reset_all()
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
    async fn reset_keybindings_reverts_all_to_defaults() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        bus.execute(RebindKey {
            action: "rename".to_owned(),
            chord: "ctrl+r".to_owned(),
            defaults: default_keys(),
        })
        .await
        .unwrap();
        bus.execute(RebindKey {
            action: "new-session".to_owned(),
            chord: "ctrl+t".to_owned(),
            defaults: default_keys(),
        })
        .await
        .unwrap();

        bus.execute(ResetKeybindings {
            defaults: default_keys(),
        })
        .await
        .unwrap();

        let rename = bus
            .query(ResolveKeybinding {
                action: "rename".to_owned(),
                defaults: default_keys(),
            })
            .await
            .unwrap();
        let new_sess = bus
            .query(ResolveKeybinding {
                action: "new-session".to_owned(),
                defaults: default_keys(),
            })
            .await
            .unwrap();

        assert_eq!(rename.as_deref(), Some("F2"));
        assert_eq!(new_sess.as_deref(), Some("ctrl+n"));
    }
}
