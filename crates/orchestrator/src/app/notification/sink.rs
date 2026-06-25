use crate::app::notification::RecordNotification;
use crate::shared::bus::Broadcast;

pub trait NotificationSink: Send + Sync + 'static {
    fn emit(&self, notification: &RecordNotification);
}

impl NotificationSink for Broadcast<dyn NotificationSink> {
    fn emit(&self, notification: &RecordNotification) {
        self.dispatch(|s| s.emit(notification));
    }
}

impl<F> NotificationSink for F
where
    F: Fn(&RecordNotification) + Send + Sync + 'static,
{
    fn emit(&self, notification: &RecordNotification) {
        self(notification)
    }
}
