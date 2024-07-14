use std::io::Write;

use anyhow::{Context, Result};
use chrono::Local;

use crate::time_entry::TimeEntry;

/// Consoleにtime entryを表示するためのtrait。
pub trait ConsolePresenter {
    /// タイムエントリーを表示する。
    ///
    /// # Arguments
    ///
    /// * `time_entries` - 表示するタイムエントリー
    fn show_time_entries(&mut self, time_entries: &[TimeEntry]) -> Result<()>;
}

/// タイムエントリーをMarkdownのlist形式で表示する。
pub struct ConsoleMarkdownList<'a, W: Write> {
    writer: &'a mut W,
}

impl<'a, W: Write> ConsoleMarkdownList<'a, W> {
    /// 新しい`ConsoleMarkdownList`を返す。
    pub fn new(writer: &'a mut W) -> Self {
        Self { writer }
    }
}

impl<'a, W: Write> ConsolePresenter for ConsoleMarkdownList<'a, W> {
    // time entryをlist形式で表示する。
    fn show_time_entries(&mut self, time_entries: &[TimeEntry]) -> Result<()> {
        let mut sorted_entries = time_entries.to_vec();
        sorted_entries.sort_by_key(|entry| entry.start);

        for entry in sorted_entries {
            let start_str = entry
                .start
                .with_timezone(&Local)
                .format("%H:%M")
                .to_string();
            let end_str = entry
                .stop
                .map(|stop| stop.with_timezone(&Local).format("%H:%M").to_string())
                .unwrap_or_else(|| "now".to_string());
            writeln!(
                self.writer,
                "- {} ~ {}: {}",
                start_str, end_str, entry.description
            )
            .with_context(|| format!("Failed to write time entry: {:?}", entry))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::ConsoleMarkdownList;
    use super::ConsolePresenter;
    use crate::time_entry::TimeEntry;
    use anyhow::Result;
    use chrono::{TimeZone, Utc};

    /// 正常系のテスト。
    #[test]
    fn test_show_time_entries() -> Result<()> {
        let mut writer = Vec::new();
        let mut presenter = ConsoleMarkdownList::new(&mut writer);

        let entry1_start_utc = Utc.with_ymd_and_hms(2021, 1, 1, 10, 0, 0).unwrap();
        let entry1_stop_utc = Utc.with_ymd_and_hms(2021, 1, 1, 11, 0, 0).unwrap();
        let entry2_start_utc = Utc.with_ymd_and_hms(2021, 1, 1, 12, 0, 0).unwrap();
        let entry2_stop_utc = Utc.with_ymd_and_hms(2021, 1, 1, 13, 0, 0).unwrap();
        let time_entries = vec![
            TimeEntry {
                description: "entry1".to_string(),
                start: entry1_start_utc,
                stop: Some(entry1_stop_utc),
                duration: 3600,
                project: None,
                tags: vec![],
            },
            TimeEntry {
                description: "entry2".to_string(),
                start: entry2_start_utc,
                stop: Some(entry2_stop_utc),
                duration: 3600,
                project: None,
                tags: vec![],
            },
        ];

        presenter.show_time_entries(&time_entries)?;

        let entry1_start_expected = entry1_start_utc
            .with_timezone(&chrono::Local)
            .format("%H:%M");
        let entry1_stop_expected = entry1_stop_utc
            .with_timezone(&chrono::Local)
            .format("%H:%M");
        let entry2_start_expected = entry2_start_utc
            .with_timezone(&chrono::Local)
            .format("%H:%M");
        let entry2_stop_expected = entry2_stop_utc
            .with_timezone(&chrono::Local)
            .format("%H:%M");
        let expected = format!(
            "- {} ~ {}: entry1\n- {} ~ {}: entry2\n",
            entry1_start_expected,
            entry1_stop_expected,
            entry2_start_expected,
            entry2_stop_expected
        );
        assert_eq!(String::from_utf8(writer)?, expected);

        Ok(())
    }

    /// 入力が空の場合は何も出力せず正常終了。
    #[test]
    fn test_show_no_time_entries() -> Result<()> {
        let mut writer = Vec::new();
        let mut presenter = ConsoleMarkdownList::new(&mut writer);

        let time_entries = vec![];

        presenter.show_time_entries(&time_entries)?;

        let expected = "";
        assert_eq!(String::from_utf8(writer)?, expected);

        Ok(())
    }

    /// 結果が時刻でソートされることを確認する。
    #[test]
    fn test_show_sorted_time_entries() -> Result<()> {
        let mut writer = Vec::new();
        let mut presenter = ConsoleMarkdownList::new(&mut writer);

        let entry1_start_utc = Utc.with_ymd_and_hms(2021, 1, 1, 13, 0, 0).unwrap();
        let entry1_stop_utc = Utc.with_ymd_and_hms(2021, 1, 1, 14, 0, 0).unwrap();
        let entry2_start_utc = Utc.with_ymd_and_hms(2021, 1, 1, 12, 0, 0).unwrap();
        let entry2_stop_utc = Utc.with_ymd_and_hms(2021, 1, 1, 13, 0, 0).unwrap();
        let time_entries = vec![
            TimeEntry {
                description: "entry1".to_string(),
                start: entry1_start_utc,
                stop: Some(entry1_stop_utc),
                duration: 3600,
                project: None,
                tags: vec![],
            },
            TimeEntry {
                description: "entry2".to_string(),
                start: entry2_start_utc,
                stop: Some(entry2_stop_utc),
                duration: 3600,
                project: None,
                tags: vec![],
            },
        ];

        presenter.show_time_entries(&time_entries)?;

        let expected = "- 21:00 ~ 22:00: entry2\n- 22:00 ~ 23:00: entry1\n";
        assert_eq!(String::from_utf8(writer)?, expected);

        Ok(())
    }

    /// 同一の開始時刻の場合は、ソートされないことを確認。
    #[test]
    fn test_show_same_time_entries() -> Result<()> {
        let mut writer = Vec::new();
        let mut presenter = ConsoleMarkdownList::new(&mut writer);

        let entry1_start_utc = Utc.with_ymd_and_hms(2021, 1, 1, 3, 0, 0).unwrap();
        let entry1_stop_utc = Utc.with_ymd_and_hms(2021, 1, 1, 5, 0, 0).unwrap();
        let entry2_start_utc = Utc.with_ymd_and_hms(2021, 1, 1, 3, 0, 0).unwrap();
        let entry2_stop_utc = Utc.with_ymd_and_hms(2021, 1, 1, 4, 0, 0).unwrap();
        let time_entries = vec![
            TimeEntry {
                description: "entry1".to_string(),
                start: entry1_start_utc,
                stop: Some(entry1_stop_utc),
                duration: 3600,
                project: None,
                tags: vec![],
            },
            TimeEntry {
                description: "entry2".to_string(),
                start: entry2_start_utc,
                stop: Some(entry2_stop_utc),
                duration: 3600,
                project: None,
                tags: vec![],
            },
        ];

        presenter.show_time_entries(&time_entries)?;

        let expected = "- 12:00 ~ 14:00: entry1\n- 12:00 ~ 13:00: entry2\n";
        assert_eq!(String::from_utf8(writer)?, expected);

        Ok(())
    }
}
