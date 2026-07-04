use serde::Serialize;

/// Flat read model for a surface row. Serializes to the surface wire shape
/// (camelCase keys) and decodes straight from a row via `query_as`.
///
/// `kind` and `status` are the stored string columns (`terminal`/`diff` and
/// `pending`/`live`/`idle`/`failed`); the read path needs no enum round-trip.
///
/// `spawned_at` is the millis at which the row's PTY was last confirmed
/// running (elapsed-since-spawn in the panel title, ui-panel-compound spec);
/// `None` before any spawn has ever been confirmed for this row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, sqlx::FromRow)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct SurfaceView {
    pub id: String,
    pub session_id: String,
    pub kind: String,
    pub cwd: Option<String>,
    pub status: String,
    pub placement: Option<String>,
    pub spawned_at: Option<i64>,
}

impl SurfaceView {
    /// True when the surface's PTY is running in the daemon.
    pub fn is_live(&self) -> bool {
        self.status == "live"
    }
}
