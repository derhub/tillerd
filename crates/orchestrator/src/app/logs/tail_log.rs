use serde::Deserialize;

use crate::app::logs::parse::parse_record;
use crate::app::logs::view::LogTailView;
use crate::context::Ctx;
use crate::shared::fs;
use crate::shared::message::Query;
use crate::shared::Result;

/// A bounded window of parsed records from `path` over `[from, from + max_bytes)`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TailLog {
    pub path: String,
    pub from: u64,
    pub max_bytes: u64,
    pub align: bool,
}

impl Query<Ctx> for TailLog {
    type Out = LogTailView;
    async fn handle(&self, _cx: &Ctx) -> Result<Self::Out> {
        let tail = fs::tail(&self.path, self.from, self.max_bytes, self.align).await?;
        let records = tail.lines.iter().filter_map(|l| parse_record(l)).collect();
        Ok(LogTailView {
            records,
            start: tail.start,
            end: tail.end,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::logs::test_util::make_ctx;
    use crate::shared::bus::Bus;
    use serde_json::json;
    use std::io::Write;

    fn rec(message: &str) -> String {
        json!({ "timestamp": "t", "level": "INFO", "fields": { "message": message } }).to_string()
    }

    #[tokio::test]
    async fn parses_each_line_dropping_blanks_and_carries_offsets() {
        let dir = tempfile::TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);
        let log = dir.path().join("a.log");
        let a = rec("first");
        let b = rec("second");
        let mut f = std::fs::File::create(&log).unwrap();
        writeln!(f, "{a}").unwrap();
        writeln!(f).unwrap();
        writeln!(f, "{b}").unwrap();
        f.flush().unwrap();

        let view = bus
            .query(TailLog {
                path: log.to_str().unwrap().to_owned(),
                from: 0,
                max_bytes: 1 << 20,
                align: false,
            })
            .await
            .unwrap();

        let bodies: Vec<&str> = view.records.iter().map(|r| r.body.as_str()).collect();
        assert_eq!(bodies, vec!["first", "second"]);
        assert_eq!(view.start, 0);
        assert_eq!(view.end as usize, a.len() + 1 + 1 + b.len() + 1);
    }

    #[tokio::test]
    async fn missing_file_yields_no_records() {
        let dir = tempfile::TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        let view = bus
            .query(TailLog {
                path: "/nonexistent/zzz.log".to_owned(),
                from: 0,
                max_bytes: 1024,
                align: false,
            })
            .await
            .unwrap();

        assert!(view.records.is_empty());
        assert_eq!(view.end, 0);
    }
}
