use serde::Deserialize;

use crate::app::notification::NotificationView;
use crate::context::Ctx;
use crate::shared::message::Query;
use crate::shared::pagination::{Listing, Page};
use crate::shared::{Error, Result};

/// Unread notifications ordered by `ts DESC`, with optional pagination.
///
/// Pagination mode is rebuilt from the primitive wire inputs in `handle`:
/// `after` present -> cursor page; else `limit` present -> offset page; else all.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListUnreadNotifications {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub after: Option<String>,
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

impl Query<Ctx> for ListUnreadNotifications {
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
                     FROM notification
                     WHERE read = 0 AND (snooze_until IS NULL OR snooze_until <= ?)
                     ORDER BY ts DESC",
                )
                .bind(now_millis())
                .fetch_all(cx.db())
                .await?;
                Ok(Listing::new(items, None))
            }

            Page::Offset { limit, offset } => {
                let rows = sqlx::query_as::<_, NotificationView>(
                    "SELECT id, category, severity, title, message, detail, ts,
                            session_id, surface_id
                     FROM notification
                     WHERE read = 0 AND (snooze_until IS NULL OR snooze_until <= ?)
                     ORDER BY ts DESC LIMIT ? OFFSET ?",
                )
                .bind(now_millis())
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
                         FROM notification
                         WHERE read = 0 AND (snooze_until IS NULL OR snooze_until <= ?) AND ts < ?
                         ORDER BY ts DESC, id DESC LIMIT ?",
                    )
                    .bind(now_millis())
                    .bind(cursor_ts)
                    .bind(limit as i64 + 1)
                    .fetch_all(cx.db())
                    .await?
                } else {
                    sqlx::query_as::<_, NotificationView>(
                        "SELECT id, category, severity, title, message, detail, ts,
                                session_id, surface_id
                         FROM notification
                         WHERE read = 0 AND (snooze_until IS NULL OR snooze_until <= ?)
                         ORDER BY ts DESC, id DESC LIMIT ?",
                    )
                    .bind(now_millis())
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
    use crate::app::notification::snooze_notification::SnoozeNotification;
    use crate::app::notification::test_util::*;
    use crate::shared::Bus;

    #[tokio::test]
    async fn snoozed_into_the_future_is_excluded_until_it_elapses() {
        let bus = Bus::new(test_ctx().await);
        bus.execute(record_cmd("future")).await.unwrap();
        bus.execute(record_cmd("past")).await.unwrap();
        bus.execute(record_cmd("plain")).await.unwrap();

        let now = now_millis();
        bus.execute(SnoozeNotification {
            id: "future".into(),
            snooze_until: Some(now + 60_000),
        })
        .await
        .unwrap();
        bus.execute(SnoozeNotification {
            id: "past".into(),
            snooze_until: Some(now - 60_000),
        })
        .await
        .unwrap();

        let listing = bus.query(list_unread_all()).await.unwrap();
        let ids: Vec<&str> = listing.items.iter().map(|n| n.id.as_str()).collect();

        // future-snoozed is excluded; past-snoozed (elapsed) and plain both show.
        assert!(!ids.contains(&"future"));
        assert!(ids.contains(&"past"));
        assert!(ids.contains(&"plain"));
    }
}
