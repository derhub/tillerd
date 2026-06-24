use serde::Deserialize;

use crate::app::notification::NotificationView;
use crate::context::Ctx;
use crate::shared::message::Query;
use crate::shared::pagination::{Listing, Page};
use crate::shared::{Error, Result};

// Projection columns for the `NotificationView` read model. Internal columns
// (`actions_json`, `read`, `snooze_until`) are not on the wire and so are not selected.

/// All notifications ordered by `ts DESC`, with optional pagination.
///
/// Pagination mode is rebuilt from the primitive wire inputs in `handle`:
/// `after` present -> cursor page; else `limit` present -> offset page; else all.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListNotifications {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub after: Option<String>,
}

impl Query<Ctx> for ListNotifications {
    type Out = Listing<NotificationView>;

    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        let page = if self.after.is_some() {
            Page::Cursor {
                after: self.after.clone(),
                limit: self.limit.unwrap_or(0),
            }
        } else if let Some(limit) = self.limit {
            Page::Offset {
                limit,
                offset: self.offset.unwrap_or(0),
            }
        } else {
            Page::All
        };

        match page {
            Page::All => {
                let items = sqlx::query_as::<_, NotificationView>(
                    "SELECT id, category, severity, title, message, detail, ts,
                            session_id, surface_id
                     FROM notification ORDER BY ts DESC",
                )
                .fetch_all(cx.db())
                .await?;
                Ok(Listing::new(items, None))
            }

            Page::Offset { limit, offset } => {
                let rows = sqlx::query_as::<_, NotificationView>(
                    "SELECT id, category, severity, title, message, detail, ts,
                            session_id, surface_id
                     FROM notification ORDER BY ts DESC LIMIT ? OFFSET ?",
                )
                .bind(limit as i64 + 1)
                .bind(offset as i64)
                .fetch_all(cx.db())
                .await?;

                let has_more = rows.len() > limit as usize;
                let items: Vec<NotificationView> = rows.into_iter().take(limit as usize).collect();
                let next = if has_more {
                    Some((offset + limit).to_string())
                } else {
                    None
                };
                Ok(Listing::new(items, next))
            }

            Page::Cursor { after, limit } => {
                // Cursor is the `ts` of the last item on the previous page.
                let rows = if let Some(cursor) = &after {
                    let cursor_ts: i64 = cursor.parse().map_err(|_| Error::Validation {
                        field: "cursor",
                        reason: "not a valid timestamp cursor".to_owned(),
                    })?;
                    sqlx::query_as::<_, NotificationView>(
                        "SELECT id, category, severity, title, message, detail, ts,
                                session_id, surface_id
                         FROM notification WHERE ts < ? ORDER BY ts DESC, id DESC LIMIT ?",
                    )
                    .bind(cursor_ts)
                    .bind(limit as i64 + 1)
                    .fetch_all(cx.db())
                    .await?
                } else {
                    sqlx::query_as::<_, NotificationView>(
                        "SELECT id, category, severity, title, message, detail, ts,
                                session_id, surface_id
                         FROM notification ORDER BY ts DESC, id DESC LIMIT ?",
                    )
                    .bind(limit as i64 + 1)
                    .fetch_all(cx.db())
                    .await?
                };

                let has_more = rows.len() > limit as usize;
                let items: Vec<NotificationView> = rows.into_iter().take(limit as usize).collect();
                let next = if has_more {
                    items.last().map(|r| r.ts.to_string())
                } else {
                    None
                };
                Ok(Listing::new(items, next))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::notification::count_unread_notifications::CountUnreadNotifications;
    use crate::app::notification::test_util::*;
    use crate::shared::Bus;

    #[tokio::test]
    async fn list_notifications_does_not_mutate_state() {
        let bus = Bus::new(test_ctx().await);
        bus.execute(record_cmd("a")).await.unwrap();
        bus.execute(record_cmd("b")).await.unwrap();

        // two queries in a row -- count must not change
        let c1 = bus.query(CountUnreadNotifications).await.unwrap();
        let c2 = bus.query(CountUnreadNotifications).await.unwrap();
        assert_eq!(c1, c2);
        assert_eq!(c1, 2);
    }

    #[tokio::test]
    async fn list_notifications_ordered_by_ts_desc() {
        let bus = Bus::new(test_ctx().await);
        for (id, ts) in [("q1", 100i64), ("q2", 300), ("q3", 200)] {
            bus.execute(record_cmd_at(id, ts)).await.unwrap();
        }

        let listing = bus.query(list_all()).await.unwrap();
        let ids: Vec<&str> = listing.items.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["q2", "q3", "q1"]);
    }

    #[tokio::test]
    async fn offset_page_returns_bounded_slice_and_continuation() {
        let bus = Bus::new(test_ctx().await);
        for (id, ts) in [("o1", 10), ("o2", 20), ("o3", 30), ("o4", 40), ("o5", 50)] {
            bus.execute(record_cmd_at(id, ts)).await.unwrap();
        }
        // ts DESC = [o5, o4, o3, o2, o1]
        let page1 = bus
            .query(ListNotifications {
                limit: Some(2),
                offset: Some(0),
                after: None,
            })
            .await
            .unwrap();
        assert_eq!(page1.items.len(), 2);
        assert_eq!(page1.items[0].id, "o5");
        assert_eq!(page1.items[1].id, "o4");
        assert!(page1.next.is_some());

        let page3 = bus
            .query(ListNotifications {
                limit: Some(2),
                offset: Some(4),
                after: None,
            })
            .await
            .unwrap();
        assert_eq!(page3.items.len(), 1);
        assert!(page3.next.is_none());
    }

    #[tokio::test]
    async fn cursor_page_from_start_then_after() {
        let bus = Bus::new(test_ctx().await);
        for (id, ts) in [("c1", 10), ("c2", 20), ("c3", 30)] {
            bus.execute(record_cmd_at(id, ts)).await.unwrap();
        }
        let page1 = bus
            .query(ListNotifications {
                limit: Some(2),
                offset: None,
                after: Some("99999".to_owned()),
            })
            .await
            .unwrap();
        assert_eq!(page1.items.len(), 2);
        assert_eq!(page1.items[0].id, "c3");
        assert_eq!(page1.items[1].id, "c2");
        let next = page1.next.expect("must carry a continuation cursor");

        let page2 = bus
            .query(ListNotifications {
                limit: Some(2),
                offset: None,
                after: Some(next),
            })
            .await
            .unwrap();
        assert_eq!(page2.items.len(), 1);
        assert_eq!(page2.items[0].id, "c1");
        assert!(page2.next.is_none());
    }
}
