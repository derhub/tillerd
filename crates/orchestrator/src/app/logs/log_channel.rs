use crate::context::Ctx;
use crate::shared::domain_channel::{CloseDomainChannel, DomainChannelSink, OpenDomainChannel};
use crate::shared::Result;

#[derive(Debug, serde::Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct OpenLogChannel {
    pub service: String,
}

impl OpenDomainChannel<Ctx> for OpenLogChannel {
    async fn handle(&self, cx: &Ctx, sink: std::sync::Arc<dyn DomainChannelSink>) -> Result<()> {
        cx.domain_channel_sinks()
            .register(&format!("logs://{}", self.service), sink);
        Ok(())
    }
}

#[derive(Debug, serde::Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct CloseLogChannel {
    pub service: String,
}

impl CloseDomainChannel<Ctx> for CloseLogChannel {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        cx.domain_channel_sinks()
            .remove_key(&format!("logs://{}", self.service));
        Ok(())
    }
}

#[derive(Debug, serde::Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct OpenLogsChangedChannel {
    pub channel_id: String,
}

impl OpenDomainChannel<Ctx> for OpenLogsChangedChannel {
    async fn handle(&self, cx: &Ctx, sink: std::sync::Arc<dyn DomainChannelSink>) -> Result<()> {
        cx.domain_channel_sinks()
            .register(&format!("logs://changed/{}", self.channel_id), sink);
        Ok(())
    }
}

#[derive(Debug, serde::Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct CloseLogsChangedChannel {
    pub channel_id: String,
}

impl CloseDomainChannel<Ctx> for CloseLogsChangedChannel {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        cx.domain_channel_sinks()
            .remove_key(&format!("logs://changed/{}", self.channel_id));
        Ok(())
    }
}
