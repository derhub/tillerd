use serde::{Deserialize, Serialize};

use crate::context::Ctx;
use crate::entities::notification::NotificationRecord;
use crate::infra::NotificationRepo;
use crate::shared::cqs::Query;
use crate::shared::pagination::{Listing, Page};
use crate::shared::Result;

/// Unread notifications ordered by `ts DESC`, with optional pagination.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ListUnreadNotifications {
    pub page: Page,
}

impl Query<Ctx> for ListUnreadNotifications {
    type Out = Listing<NotificationRecord>;

    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        NotificationRepo::list_unread(cx.db(), &self.page).await
    }
}
