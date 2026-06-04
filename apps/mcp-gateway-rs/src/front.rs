//! Holder for the current front-client peer.
//!
//! Backends issue server-to-client requests (sampling, roots, elicitation)
//! asynchronously, outside any front request. The gateway captures the front
//! peer when a session initializes so those requests can be relayed. For v1 a
//! single most-recent front peer is tracked (the primary consumer).

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
