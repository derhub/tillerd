use crate::app::notification::RecordNotification;
use crate::context::Ctx;
use crate::shared::bus::Broadcast;
use crate::shared::domain_channel::{CloseDomainChannel, DomainChannelSink, OpenDomainChannel};
use crate::shared::Result;

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

#[derive(Debug, serde::Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct OpenNotificationChannel {
    pub channel_id: String,
}

impl OpenDomainChannel<Ctx> for OpenNotificationChannel {
    async fn handle(&self, cx: &Ctx, sink: std::sync::Arc<dyn DomainChannelSink>) -> Result<()> {
        cx.domain_channel_sinks()
            .register(&format!("notifications://{}", self.channel_id), sink);
        Ok(())
    }
}

#[derive(Debug, serde::Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct CloseNotificationChannel {
    pub channel_id: String,
}

impl CloseDomainChannel<Ctx> for CloseNotificationChannel {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        cx.domain_channel_sinks()
            .remove_key(&format!("notifications://{}", self.channel_id));
        Ok(())
    }
}
