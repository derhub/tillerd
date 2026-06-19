use super::*;

impl MemoryBackend {
    pub(crate) fn insert_notification(&self, rec: &NotificationRecord) -> Result<()> {
        self.inner.lock().unwrap().notifications.push(rec.clone());
        Ok(())
    }

    pub(crate) fn list_notifications(&self, limit: u32) -> Result<Vec<NotificationRecord>> {
        let inner = self.inner.lock().unwrap();
        Ok(inner
            .notifications
            .iter()
            .rev()
            .take(limit as usize)
            .cloned()
            .collect())
    }

    pub(crate) fn prune_notifications(&self, keep: u32) -> Result<()> {
        let mut inner = self.inner.lock().unwrap();
        let len = inner.notifications.len();
        let keep = keep as usize;
        if len > keep {
            inner.notifications.drain(0..len - keep);
        }
        Ok(())
    }
}
