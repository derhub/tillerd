//! The file-backed logs domain. Queries (`ListLogFiles`/`TailLog`) compose
//! `shared::fs::tail` over the runtime `.log` files and parse each line into a
//! `LogRecordView`; no store, no SQL. The live side (`LogFollower` +
//! `SubscribeLogs`/`UnsubscribeLogs`) follows the logs directory with `notify`
//! and fans appended lines, borrowed and keyed by service, to registered
//! `LogSink`s.

pub(crate) mod follow;
mod list_log_files;
pub mod log_channel;
mod parse;
mod tail_log;
mod view;

#[cfg(test)]
pub(crate) mod test_util;

pub use follow::LogFollower;
pub use list_log_files::ListLogFiles;
pub use log_channel::{
    CloseLogChannel, CloseLogsChangedChannel, OpenLogChannel, OpenLogsChangedChannel,
};
pub use parse::parse_record;
pub use tail_log::TailLog;
pub use view::{LogFileView, LogRecordView, LogTailView};
