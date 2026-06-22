//! Notification entity: a durably-stored user-facing notification (ADR-0031).

/// `ts` is event time in epoch milliseconds; `actions_json` is a JSON-encoded action
/// list when present. `snooze_until` is epoch milliseconds; `None` means not snoozed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationRecord {
    pub id: String,
    pub category: String,
    pub severity: String,
    pub title: Option<String>,
    pub message: String,
    pub detail: Option<String>,
    pub ts: i64,
    pub session_id: Option<String>,
    pub surface_id: Option<String>,
    pub actions_json: Option<String>,
    /// Whether the notification has been read.
    pub read: bool,
    /// Epoch milliseconds; suppresses the record from the active list until this time passes.
    pub snooze_until: Option<i64>,
}
