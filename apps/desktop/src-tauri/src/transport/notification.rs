use std::time::{SystemTime, UNIX_EPOCH};

use orchestrator::app::notification::{
    CountUnreadNotifications, DisregardAllNotifications, DisregardNotification, ListNotifications,
    ListUnreadNotifications, MarkAllNotificationsRead, MarkNotificationRead, NotificationSink,
    NotificationView, PruneNotifications, RecordNotification, SnoozeNotification,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};

use crate::transport::macros::{domain_channel, transport_command, transport_query};

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct NotificationWire {
    pub id: String,
    pub category: String,
    pub severity: String,
    pub title: Option<String>,
    pub message: String,
    pub detail: Option<String>,
    pub ts: i64,
    pub session_id: Option<String>,
    pub surface_id: Option<String>,
}

impl NotificationWire {
    fn from_record(n: &RecordNotification) -> Self {
        Self {
            id: n.id.clone(),
            category: n.category.clone(),
            severity: n.severity.clone(),
            title: n.title.clone(),
            message: n.message.clone(),
            detail: n.detail.clone(),
            ts: n.ts,
            session_id: n.session_id.clone(),
            surface_id: n.surface_id.clone(),
        }
    }

    pub fn from_view(view: NotificationView) -> Self {
        Self {
            id: view.id,
            category: view.category,
            severity: view.severity,
            title: view.title,
            message: view.message,
            detail: view.detail,
            ts: view.ts,
            session_id: view.session_id,
            surface_id: view.surface_id,
        }
    }
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub struct NotificationForwarder<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> NotificationForwarder<R> {
    pub fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }
}

impl<R: Runtime> NotificationSink for NotificationForwarder<R> {
    fn emit(&self, notification: &RecordNotification) {
        if let Some(bus) = self.app.try_state::<crate::transport::Bus>() {
            let wire = NotificationWire::from_record(notification);
            if let Ok(json) = serde_json::to_vec(&wire) {
                let event = orchestrator::shared::domain_channel::DomainChannelEvent::Bytes(&json);
                bus.cx()
                    .domain_channel_sinks()
                    .dispatch_prefix("notifications://", |sink| {
                        sink.emit(&event);
                    });
            }
        }
    }
}

domain_channel! {
    pub open notification_channel(orchestrator::app::notification::OpenNotificationChannel),
    pub close notification_channel_close(orchestrator::app::notification::CloseNotificationChannel)
}

transport_query!(
    notification_list_unread(limit: Option<u32>, offset: Option<u32>, after: Option<String>) -> orchestrator::shared::pagination::Listing<NotificationView>
        => ListUnreadNotifications { limit, offset, after },
        |listing| listing
);

transport_query!(
    notification_count_unread() -> i64
        => CountUnreadNotifications,
        |count| count
);

transport_command!(notification_mark_read(id: String) => MarkNotificationRead { id });

transport_command!(notification_mark_all_read() => MarkAllNotificationsRead);

transport_command!(notification_disregard(id: String) => DisregardNotification { id });

transport_command!(notification_disregard_all() => DisregardAllNotifications);

transport_command!(notification_snooze(id: String, snooze_until: Option<i64>) => SnoozeNotification {
    id,
    snooze_until,
});

transport_command!(notification_prune(keep: u32) => PruneNotifications { keep });

transport_command!(
    notification_record(
        id: String,
        category: String,
        severity: String,
        title: Option<String>,
        message: String,
        detail: Option<String>,
        ts: i64,
        session_id: Option<String>,
        surface_id: Option<String>,
        actions_json: Option<String>,
        read: bool,
        snooze_until: Option<i64>,
    ) => RecordNotification {
        id,
        category,
        severity,
        title,
        message,
        detail,
        ts,
        session_id,
        surface_id,
        actions_json,
        read,
        snooze_until,
    }
);

transport_query!(
    notifications_list() -> Vec<NotificationWire>
        => ListNotifications { limit: Some(200), offset: Some(0), after: None },
        |listing| listing
            .items
            .into_iter()
            .map(NotificationWire::from_view)
            .collect()
);

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_keys(value: &serde_json::Value, expected: &[&str]) {
        let obj = value.as_object().expect("response serializes to an object");
        let mut got: Vec<&str> = obj.keys().map(String::as_str).collect();
        got.sort_unstable();
        let mut want = expected.to_vec();
        want.sort_unstable();
        assert_eq!(got, want, "response keys drifted from the SDK contract");
    }

    #[test]
    fn from_record_carries_session_and_surface() {
        let record = RecordNotification {
            id: "n1".to_owned(),
            category: "surface-started".to_owned(),
            severity: "info".to_owned(),
            title: Some("Terminal started".to_owned()),
            message: "A terminal started".to_owned(),
            detail: None,
            ts: 7,
            session_id: Some("sess-1".to_owned()),
            surface_id: Some("surf-1".to_owned()),
            actions_json: None,
            read: false,
            snooze_until: None,
        };

        let wire = NotificationWire::from_record(&record);

        assert_eq!(wire.category, "surface-started");
        assert_eq!(wire.severity, "info");
        assert_eq!(wire.session_id.as_deref(), Some("sess-1"));
        assert_eq!(wire.surface_id.as_deref(), Some("surf-1"));
        assert_eq!(wire.ts, 7);
    }

    #[test]
    fn notification_response_matches_sdk_notification_shape() {
        let n = NotificationView {
            id: "n".into(),
            category: "surface-started".into(),
            severity: "info".into(),
            title: None,
            message: "msg".into(),
            detail: None,
            ts: 0,
            session_id: None,
            surface_id: None,
        };
        assert_keys(
            &serde_json::to_value(n).unwrap(),
            &[
                "id",
                "category",
                "severity",
                "title",
                "message",
                "detail",
                "ts",
                "sessionId",
                "surfaceId",
            ],
        );
    }
}
