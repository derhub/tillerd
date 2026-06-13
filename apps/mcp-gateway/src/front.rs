//! Front peer: single most-recent (v1 limitation). Relay async server->client requests.

use rmcp::service::Peer;
use rmcp::RoleServer;
use std::sync::{Arc, RwLock};

#[derive(Clone, Default)]
pub struct FrontPeer {
    inner: Arc<RwLock<Option<Peer<RoleServer>>>>,
}

impl FrontPeer {
    pub fn set(&self, peer: Peer<RoleServer>) {
        *self.inner.write().unwrap() = Some(peer);
    }

    pub fn get(&self) -> Option<Peer<RoleServer>> {
        self.inner.read().unwrap().clone()
    }
}
