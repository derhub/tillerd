//! Notification history store (ADR-0031).

use crate::entities::NotificationRecord;
use crate::error::Result;
use crate::store::backend::Backend;

/// Operational store for the durable notification history.
#[derive(Clone)]
pub struct Notifications {
    backend: Backend,
}

impl Notifications {
    pub fn new(backend: Backend) -> Self {
        Self { backend }
    }

    pub async fn insert(&self, rec: NotificationRecord) -> Result<()> {
        self.backend.insert_notification(rec).await
    }

    /// The most recent `limit` notifications, newest first.
    pub async fn list(&self, limit: u32) -> Result<Vec<NotificationRecord>> {
        self.backend.list_notifications(limit).await
    }

    /// Retain only the most recent `keep` notifications.
    pub async fn prune(&self, keep: u32) -> Result<()> {
        self.backend.prune_notifications(keep).await
    }
}
