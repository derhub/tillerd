//! CQS queries for the file-backed logs domain. Read-only: queries compose
//! `shared::fs::tail` over the runtime `.log` files and parse each line into a
//! `LogRecordView`. No store, no SQL.

mod list_log_files;
mod parse;
mod tail_log;
mod view;

#[cfg(test)]
pub(crate) mod test_util;

pub use list_log_files::ListLogFiles;
pub use parse::parse_record;
pub use tail_log::TailLog;
pub use view::{LogFileView, LogRecordView, LogTailView};
