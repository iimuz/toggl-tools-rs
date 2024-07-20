use std::{collections::HashMap, io::Write};

use anyhow::{Context, Result};
use chrono::{Local, NaiveDate};

use crate::time_entry::{ProjectDurations, TimeEntry};

/// Consoleにtime entryを表示するためのtrait。
pub trait ConsolePresenter {
    /// タイムエントリーを表示する。
    fn show_time_entries(&mut self, time_entries: &[TimeEntry]) -> Result<()>;

    // project, tagごとの集計結果を表示する。
    fn show_durations(&mut self, durations: &ProjectDurations) -> Result<()>;
    // 複数の集計結果を表示する。
    fn show_multi_durations(
        &mut self,
        durations: &HashMap<NaiveDate, ProjectDurations>,
    ) -> Result<()>;
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

    // project, tagごとの集計結果を表示する。
    fn show_durations(&mut self, durations: &ProjectDurations) -> Result<()> {
        durations.iter().for_each(|(project, tags)| {
            println!("- {}", project);
            tags.iter().for_each(|(tag, duration)| {
                let duration_hours = *duration as f64 / 3600.0;
                println!("  - {}: {:.2}", tag, duration_hours);
            });
        });

        Ok(())
    }

    // project, tagごとの集計結果を表示する。
    fn show_multi_durations(
        &mut self,
        durations: &HashMap<NaiveDate, ProjectDurations>,
    ) -> Result<()> {
        let mut sorted_durations = durations.iter().collect::<Vec<_>>();
        sorted_durations.sort_by_key(|(date, _)| *date);
        sorted_durations.iter().for_each(|(date, duration)| {
            println!("## {}", date);
            self.show_durations(duration).unwrap();
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Local, TimeZone, Utc};
    use rstest::rstest;

    use super::ConsoleMarkdownList;
    use super::ConsolePresenter;
    use crate::time_entry::TimeEntry;

    /// 正常系のテスト。
    #[rstest]
    #[case::no_entry(&[], "")]
    #[case::single(
        &[dummy_entry(1)],
        &expected_output(&dummy_entry(1)),
    )]
    #[case::no_stop(
        &[dummy_entry(4)],
        &expected_output(&dummy_entry(4)),
    )]
    #[case::double(
        &[dummy_entry(1), dummy_entry(2)],
        &[expected_output(&dummy_entry(1)),expected_output(&dummy_entry(2))].join(""),
    )]
    #[case::sort_with_start_time(
        &[dummy_entry(2), dummy_entry(1)],
        &[expected_output(&dummy_entry(1)),expected_output(&dummy_entry(2))].join(""),
    )]
    #[case::no_sort_with_same_start_time(
        &[dummy_entry(3), dummy_entry(2)],
        &[expected_output(&dummy_entry(3)),expected_output(&dummy_entry(2))].join(""),
    )]
    fn test_show_time_entries(#[case] input: &[TimeEntry], #[case] expected: &str) {
        let mut writer = Vec::new();
        let mut presenter = ConsoleMarkdownList::new(&mut writer);

        presenter.show_time_entries(input).unwrap();

        assert_eq!(String::from_utf8(writer).unwrap(), expected);
    }

    /// テスト用にダミーのTimeEntryを作成する。
    fn dummy_entry(pattern: u8) -> TimeEntry {
        match pattern {
            1 => TimeEntry {
                description: "entry1".to_string(),
                start: Utc.with_ymd_and_hms(2021, 1, 1, 1, 0, 0).unwrap(),
                stop: Some(Utc.with_ymd_and_hms(2021, 1, 1, 2, 0, 0).unwrap()),
                duration: 3600, // 利用しないのでなんでも良い
                project: None,  // 利用しないのでなんでも良い
                tags: vec![],   // 利用しないのでなんでも良い
            },
            2 => TimeEntry {
                description: "entry2".to_string(),
                start: Utc.with_ymd_and_hms(2021, 1, 1, 3, 0, 0).unwrap(),
                stop: Some(Utc.with_ymd_and_hms(2021, 1, 1, 4, 0, 0).unwrap()),
                duration: 3600, // 利用しないのでなんでも良い
                project: None,  // 利用しないのでなんでも良い
                tags: vec![],   // 利用しないのでなんでも良い
            },
            3 => TimeEntry {
                description: "entry3".to_string(),
                start: Utc.with_ymd_and_hms(2021, 1, 1, 3, 0, 0).unwrap(),
                stop: Some(Utc.with_ymd_and_hms(2021, 1, 1, 5, 0, 0).unwrap()),
                duration: 7200, // 利用しないのでなんでも良い
                project: None,  // 利用しないのでなんでも良い
                tags: vec![],   // 利用しないのでなんでも良い
            },
            4 => TimeEntry {
                description: "entry3".to_string(),
                start: Utc.with_ymd_and_hms(2021, 1, 1, 5, 0, 0).unwrap(),
                stop: None,
                duration: 7200, // 利用しないのでなんでも良い
                project: None,  // 利用しないのでなんでも良い
                tags: vec![],   // 利用しないのでなんでも良い
            },
            _ => panic!("Invalid pattern: {}", pattern),
        }
    }

    /// テスト用に出力の1 time entryに対する期待値の文字列を作成する。
    fn expected_output(entry: &TimeEntry) -> String {
        let start_str = entry
            .start
            .with_timezone(&Local)
            .format("%H:%M")
            .to_string();
        let end_str = entry
            .stop
            .map(|stop| stop.with_timezone(&Local).format("%H:%M").to_string())
            .unwrap_or_else(|| "now".to_string());
        format!("- {} ~ {}: {}\n", start_str, end_str, entry.description)
    }
}
