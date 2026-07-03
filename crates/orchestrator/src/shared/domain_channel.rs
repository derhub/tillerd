pub enum DomainChannelEvent<'a> {
    Bytes(&'a [u8]),
    Status(&'a str),
    Exit(&'a str),
    Error(&'a str),
}

pub trait DomainChannelSink: Send + Sync + 'static {
    fn emit(&self, event: &DomainChannelEvent<'_>);
}

pub trait OpenDomainChannel<Cx>: Send + 'static {
    fn handle(
        &self,
        cx: &Cx,
        sink: std::sync::Arc<dyn DomainChannelSink>,
    ) -> impl std::future::Future<Output = crate::shared::Result<()>> + Send;
}

pub trait CloseDomainChannel<Cx>: Send + 'static {
    fn handle(
        &self,
        cx: &Cx,
    ) -> impl std::future::Future<Output = crate::shared::Result<()>> + Send;
}

pub trait DomainChannelMessage<Cx>: Send + 'static {
    fn handle(
        &self,
        cx: &Cx,
        key: &str,
    ) -> impl std::future::Future<Output = crate::shared::Result<()>> + Send;
}

pub trait DomainChannelStream: Send + 'static {
    fn handle(self) -> impl std::future::Future<Output = ()> + Send;
}
