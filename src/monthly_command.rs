use std::collections::HashMap;

use anyhow::{Context, Ok, Result};
use chrono::{DateTime, Datelike, Local, NaiveDate, TimeZone, Timelike, Utc};
use log::info;

use crate::datetime::now;
use crate::time_entry::{ProjectDurations, TimeEntry};
use crate::toggl::TogglRepository;

/// 月毎の情報を出力するためのサブコマンド。
#[derive(Debug, clap::Args)]
pub struct MonthlyArgs {
    #[clap(
        short = 'm',
        long = "month",
        help = "Sets a custom month in the format YYYY-MM",
        parse(try_from_str = parse_month),
    )]
    month: Option<DateTime<Utc>>,

    #[clap(long = "daily", help = "Show summary by day")]
    daily: bool,
}

impl MonthlyArgs {
    /// dailyフラグを取得する。
    pub fn get_daily(&self) -> bool {
        self.daily
    }
}

pub struct MonthlyCommand<'a, T: TogglRepository> {
    toggl_client: &'a T,
}

impl<'a, T: TogglRepository> MonthlyCommand<'a, T> {
    /// 新しい`MonthlyCommand`を返す。
    pub fn new(toggl_client: &'a T) -> Self {
        Self { toggl_client }
    }

    // monthly sub commandで月の集計情報を返す。
    pub async fn run_monthly_duration(&self, monthly: MonthlyArgs) -> Result<ProjectDurations> {
        // Localのタイムゾーンで00:00:00から始まる1日とする
        let date = monthly.month.unwrap_or_else(now);
        let (start_at, end_at) =
            calc_start_and_end_date(date).context("Failed to calculate start and end date")?;

        info!("Start at: {}, End at: {}", start_at, end_at);
        let time_entries = self
            .toggl_client
            .read_time_entries(&start_at, &end_at)
            .await
            .context("Failed to retrieve time entries")?;
        info!("Time entries retrieved successfully.");

        let durations = calc_project_tag_duration(&time_entries);

        Ok(durations)
    }

    // monthly sub commandで日毎の集計情報を返す。
    pub async fn run_daily_duration(
        &self,
        monthly: MonthlyArgs,
    ) -> Result<HashMap<NaiveDate, ProjectDurations>> {
        // Localのタイムゾーンで00:00:00から始まる1日とする
        let date = monthly.month.unwrap_or_else(now);
        let (start_at, end_at) =
            calc_start_and_end_date(date).context("Failed to calculate start and end date")?;

        info!("Start at: {}, End at: {}", start_at, end_at);
        let time_entries = self
            .toggl_client
            .read_time_entries(&start_at, &end_at)
            .await
            .context("Failed to retrieve time entries")?;
        info!("Time entries retrieved successfully.");

        let daily_time_entries: HashMap<NaiveDate, Vec<TimeEntry>> =
            time_entries.iter().fold(HashMap::new(), |mut acc, entry| {
                let start = entry.start.with_timezone(&Local).date_naive();
                acc.entry(start).or_default().push(entry.clone());
                acc
            });
        let durations = daily_time_entries
            .iter()
            .map(|(date, entries)| {
                let result = calc_project_tag_duration(entries);
                (*date, result)
            })
            .collect::<HashMap<_, _>>();

        Ok(durations)
    }
}

/// 月をパースする。
fn parse_month(s: &str) -> Result<DateTime<Utc>> {
    let target_date = s.to_string() + "-01";
    let naive_date = NaiveDate::parse_from_str(&target_date, "%Y-%m-%d")
        .with_context(|| format!("Failed to parse date: {}", target_date))?;
    let naive_datetime = naive_date
        .with_day0(0)
        .context("Failed to set day")?
        .and_hms_opt(0, 0, 0)
        .context("Failed to set hour, minute, and second")?;
    let datetime = Local
        .from_local_datetime(&naive_datetime)
        // 環境変数を書き換えるときに並行処理した場合用のmutex
        .single()
        .context("Failed to convert to DateTime<Local>")?
        .to_utc();

    Ok(datetime)
}

/// プロジェクトごと、かつタグごとの集計結果を計算する。
///
/// 終了していないtime entryは集計対象外とする。
fn calc_project_tag_duration(time_entries: &[TimeEntry]) -> ProjectDurations {
    let project_tag_duration: ProjectDurations =
        time_entries
            .iter()
            .fold(ProjectDurations::new(), |mut acc, entry| {
                if entry.stop.is_none() {
                    return acc;
                }

                let key = entry.project.clone().unwrap_or_default();
                let project_entry = acc.entry(key).or_default();
                entry.tags.iter().for_each(|tag| {
                    *project_entry.entry(tag.clone()).or_insert(0) += entry.duration;
                });
                acc
            });

    project_tag_duration
}

// 指定した日時のlocalの日時を含む月の開始日時と終了日時を返す。
fn calc_start_and_end_date(date: DateTime<Utc>) -> Result<(DateTime<Utc>, DateTime<Utc>)> {
    let local_date = date.with_timezone(&Local);
    let start_at = local_date
        .with_day0(0)
        .context("Failed to set day")?
        .with_hour(0)
        .context("Failed to set hour")?
        .with_minute(0)
        .context("Failed to set minute")?
        .with_second(0)
        .context("Failed to set second")?;

    let end_year = if start_at.month() == 12 {
        start_at.year() + 1
    } else {
        start_at.year()
    };
    let end_month = if start_at.month() == 12 {
        1
    } else {
        start_at.month() + 1
    };
    let end_at = start_at
        .with_year(end_year)
        .context("Failed to set year")?
        .with_month(end_month)
        .context("Failed to set month")?;

    Ok((start_at.to_utc(), end_at.to_utc()))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::{DateTime, Datelike, Local, NaiveDate, NaiveDateTime, TimeZone, Timelike, Utc};
    use mockall::predicate;
    use rstest::rstest;

    use super::parse_month;
    use super::{MonthlyArgs, MonthlyCommand};
    use crate::datetime::mock_datetime;
    use crate::time_entry::{ProjectDurations, TimeEntry};
    use crate::toggl::MockTogglRepository;

    // monthの値がNoneの場合を含めて正常に動作するかテストする。
    #[tokio::test]
    #[rstest]
    #[case::none_month(None)]
    #[case::some_month(Some(DateTime::parse_from_rfc3339("2024-01-05T00:00:00+00:00").unwrap().to_utc()))]
    #[case::year_end(Some(DateTime::parse_from_rfc3339("2024-12-05T00:00:00+00:00").unwrap().to_utc()))]
    async fn test_run_monthly_duration_month_option(#[case] month: Option<DateTime<Utc>>) {
        let args = MonthlyArgs {
            month,
            daily: false,
        };
        let mut toggl = MockTogglRepository::new();

        let now = month.unwrap_or(Utc::now());
        let (start_at, end_at) = calc_start_and_end(now);
        mock_datetime::set_mock_time(now);

        let entries = vec![TimeEntry {
            description: "test 1".to_string(),
            start: start_at.with_hour(3).unwrap().to_utc(),
            stop: Some(end_at.with_hour(4).unwrap().to_utc()),
            duration: 3600,
            project: None,
            tags: vec![],
        }];
        toggl
            .expect_read_time_entries()
            .with(
                predicate::eq(start_at.to_utc()),
                predicate::eq(end_at.to_utc()),
            )
            .times(1)
            .returning(move |_, _| Ok(entries.clone()));

        let command = MonthlyCommand::new(&toggl);
        let result = command.run_monthly_duration(args).await;

        assert!(result.is_ok());
    }

    // time entriesの値による正常系のテスト。
    #[tokio::test]
    #[rstest]
    #[case::no_entry(&[])]
    #[case::no_project_no_tag_entry(&[dummy_entry(1)])]
    #[case::no_tag(&[dummy_entry(2)])]
    #[case::no_project(&[dummy_entry(3)])]
    #[case::none_stop(&[dummy_entry(8)])]
    #[case::normal(&[dummy_entry(4), dummy_entry(5), dummy_entry(6), dummy_entry(7)])]
    async fn test_run_monthly_duration_time_entries(#[case] entries: &[TimeEntry]) {
        let mut toggl = MockTogglRepository::new();

        let now = DateTime::parse_from_rfc3339("2024-01-05T04:00:00+00:00")
            .unwrap()
            .to_utc();
        let (start_at, end_at) = calc_start_and_end(now);
        let args = MonthlyArgs {
            month: Some(now),
            daily: false,
        };
        mock_datetime::set_mock_time(now);

        let expected = entries
            .iter()
            .fold(ProjectDurations::new(), |mut acc, entry| {
                // 終わっていないentryは集計対象外
                if entry.stop.is_none() {
                    return acc;
                }

                let key = entry.project.clone().unwrap_or_default();
                let project_entry = acc.entry(key).or_default();
                entry.tags.iter().for_each(|tag| {
                    *project_entry.entry(tag.clone()).or_insert(0) += entry.duration;
                });
                acc
            });
        let retuning_entries = entries.to_vec();
        toggl
            .expect_read_time_entries()
            .with(predicate::eq(start_at), predicate::eq(end_at))
            .times(1)
            .returning(move |_, _| Ok(retuning_entries.clone()));

        let command = MonthlyCommand::new(&toggl);
        let result = command.run_monthly_duration(args).await;

        assert!(result.is_ok());
        assert_eq!(expected, result.unwrap());
    }

    // time entryの失敗発生時のテスト。
    #[tokio::test]
    async fn test_run_monthly_duration_error_time_entry() {
        let mut toggl = MockTogglRepository::new();

        let now = DateTime::parse_from_rfc3339("2024-01-05T04:00:00+00:00")
            .unwrap()
            .to_utc();
        let (start_at, end_at) = calc_start_and_end(now);
        let args = MonthlyArgs {
            month: Some(now),
            daily: false,
        };
        mock_datetime::set_mock_time(now);

        toggl
            .expect_read_time_entries()
            .with(predicate::eq(start_at), predicate::eq(end_at))
            .times(1)
            .returning(move |_, _| Err(anyhow::anyhow!("Test error")));

        let command = MonthlyCommand::new(&toggl);
        let result = command.run_monthly_duration(args).await;

        assert!(result.is_err());
    }

    // monthの値がNoneの場合を含めて正常に動作するかテストする。
    #[tokio::test]
    #[rstest]
    #[case::none_month(None)]
    #[case::some_month(Some(DateTime::parse_from_rfc3339("2024-01-05T00:00:00+00:00").unwrap().to_utc()))]
    #[case::year_end(Some(DateTime::parse_from_rfc3339("2024-12-05T00:00:00+00:00").unwrap().to_utc()))]
    async fn test_run_daily_duration_month_option(#[case] month: Option<DateTime<Utc>>) {
        let args = MonthlyArgs { month, daily: true };
        let mut toggl = MockTogglRepository::new();

        let now = month.unwrap_or(Utc::now());
        let (start_at, end_at) = calc_start_and_end(now);
        mock_datetime::set_mock_time(now);

        let entries = vec![TimeEntry {
            description: "test 1".to_string(),
            start: start_at.with_hour(3).unwrap().to_utc(),
            stop: Some(end_at.with_hour(4).unwrap().to_utc()),
            duration: 3600,
            project: None,
            tags: vec![],
        }];
        toggl
            .expect_read_time_entries()
            .with(
                predicate::eq(start_at.to_utc()),
                predicate::eq(end_at.to_utc()),
            )
            .times(1)
            .returning(move |_, _| Ok(entries.clone()));

        let command = MonthlyCommand::new(&toggl);
        let result = command.run_daily_duration(args).await;

        assert!(result.is_ok());
    }

    // time entriesの値による正常系のテスト。
    #[tokio::test]
    #[rstest]
    #[case::no_entry(&[])]
    #[case::no_project_no_tag_entry(&[dummy_entry(1)])]
    #[case::no_tag(&[dummy_entry(2)])]
    #[case::no_project(&[dummy_entry(3)])]
    #[case::none_stop(&[dummy_entry(8)])]
    #[case::normal(&[dummy_entry(4), dummy_entry(5), dummy_entry(6), dummy_entry(7)])]
    async fn test_run_daily_duration_time_entries(#[case] entries: &[TimeEntry]) {
        let mut toggl = MockTogglRepository::new();

        let now = DateTime::parse_from_rfc3339("2024-01-05T04:00:00+00:00")
            .unwrap()
            .to_utc();
        let (start_at, end_at) = calc_start_and_end(now);
        let args = MonthlyArgs {
            month: Some(now),
            daily: true,
        };
        mock_datetime::set_mock_time(now);

        let daily_entries = entries.iter().fold(
            HashMap::<NaiveDate, Vec<TimeEntry>>::new(),
            |mut acc, entry| {
                let start = entry.start.with_timezone(&Local).date_naive();
                let date_entries = acc.entry(start).or_default();
                date_entries.push(entry.clone());
                acc
            },
        );
        let daily_durations = daily_entries
            .iter()
            .map(|(date, entries)| {
                let durations = entries
                    .iter()
                    .fold(ProjectDurations::new(), |mut acc, entry| {
                        // 終わっていないentryは集計対象外
                        if entry.stop.is_none() {
                            return acc;
                        }

                        let key = entry.project.clone().unwrap_or_default();
                        let project_entry = acc.entry(key).or_default();
                        entry.tags.iter().for_each(|tag| {
                            *project_entry.entry(tag.clone()).or_insert(0) += entry.duration;
                        });
                        acc
                    });
                (*date, durations)
            })
            .collect::<HashMap<_, _>>();
        let retuning_entries = entries.to_vec();
        toggl
            .expect_read_time_entries()
            .with(predicate::eq(start_at), predicate::eq(end_at))
            .times(1)
            .returning(move |_, _| Ok(retuning_entries.clone()));

        let command = MonthlyCommand::new(&toggl);
        let result = command.run_daily_duration(args).await;

        assert!(result.is_ok());
        assert_eq!(daily_durations, result.unwrap());
    }

    // time entryの失敗発生時のテスト。
    #[tokio::test]
    async fn test_run_daily_duration_error_time_entry() {
        let mut toggl = MockTogglRepository::new();

        let now = DateTime::parse_from_rfc3339("2024-01-05T04:00:00+00:00")
            .unwrap()
            .to_utc();
        let (start_at, end_at) = calc_start_and_end(now);
        let args = MonthlyArgs {
            month: Some(now),
            daily: true,
        };
        mock_datetime::set_mock_time(now);

        toggl
            .expect_read_time_entries()
            .with(predicate::eq(start_at), predicate::eq(end_at))
            .times(1)
            .returning(move |_, _| Err(anyhow::anyhow!("Test error")));

        let command = MonthlyCommand::new(&toggl);
        let result = command.run_daily_duration(args).await;

        assert!(result.is_err());
    }

    /// 正常に日付をパースできることを確認する。
    #[test]
    fn test_parse_month_valid_date() {
        let month_str = "2022-12";
        let expected_date = Local
            .from_local_datetime(
                &NaiveDateTime::parse_from_str("2022-12-01T00:00:00", "%Y-%m-%dT%H:%M:%S").unwrap(),
            )
            .unwrap()
            .to_utc();

        let result = parse_month(month_str);

        assert!(result.is_ok());
        assert_eq!(expected_date, result.unwrap());
    }

    /// 入力日付が間違っている場合にエラーを返すことを確認する。
    #[rstest]
    #[test]
    #[case::no_month("2024")]
    #[case::with_date("2024-01-01")]
    #[case::invalid_year("20xx-01")]
    #[case::invalid_month("2024-13")]
    #[case::invalid_format("2024/01")]
    #[case::empty_string("")]
    fn test_parse_month_invalid_date(#[case] date_str: &str) {
        let result = parse_month(date_str);

        assert!(result.is_err());
    }

    /// 開始日付と終了日付を月初と翌月開始日付で返す。
    ///
    /// 12月の翌月計算を行うため、年と月の繰越計算が必要。
    fn calc_start_and_end(date: DateTime<Utc>) -> (DateTime<Utc>, DateTime<Utc>) {
        let start_at = date
            .with_timezone(&Local)
            .with_day0(0)
            .unwrap()
            .with_hour(0)
            .unwrap()
            .with_minute(0)
            .unwrap()
            .with_second(0)
            .unwrap();

        let end_year = if start_at.month() == 12 {
            start_at.year() + 1
        } else {
            start_at.year()
        };
        let end_month = if start_at.month() == 12 {
            1
        } else {
            start_at.month() + 1
        };
        let end_at = start_at
            .with_year(end_year)
            .unwrap()
            .with_month(end_month)
            .unwrap();

        (start_at.to_utc(), end_at.to_utc())
    }

    /// dummyのTimeEntryを返す。
    fn dummy_entry(pattern: u8) -> TimeEntry {
        match pattern {
            // no project, no tags
            1 => TimeEntry {
                description: "entry1".to_string(),
                start: DateTime::parse_from_rfc3339("2024-01-02T01:00:00+00:00")
                    .unwrap()
                    .to_utc(),
                stop: Some(
                    DateTime::parse_from_rfc3339("2024-01-02T02:00:00+00:00")
                        .unwrap()
                        .to_utc(),
                ),
                duration: 3600,
                project: None,
                tags: vec![],
            },
            // no tags
            2 => TimeEntry {
                description: "entry2".to_string(),
                start: DateTime::parse_from_rfc3339("2024-01-03T02:00:00+00:00")
                    .unwrap()
                    .to_utc(),
                stop: Some(
                    DateTime::parse_from_rfc3339("2024-01-03T03:00:05+00:00")
                        .unwrap()
                        .to_utc(),
                ),
                duration: 3605,
                project: Some("project1".to_string()),
                tags: vec![],
            },
            // no project
            3 => TimeEntry {
                description: "entry3".to_string(),
                start: DateTime::parse_from_rfc3339("2024-01-03T04:00:00+00:00")
                    .unwrap()
                    .to_utc(),
                stop: Some(
                    DateTime::parse_from_rfc3339("2024-01-03T05:00:10+00:00")
                        .unwrap()
                        .to_utc(),
                ),
                duration: 3610,
                project: None,
                tags: vec!["tag1".to_string()],
            },
            4 => TimeEntry {
                description: "entry4".to_string(),
                start: DateTime::parse_from_rfc3339("2024-01-03T05:00:00+00:00")
                    .unwrap()
                    .to_utc(),
                stop: Some(
                    DateTime::parse_from_rfc3339("2024-01-03T06:00:15+00:00")
                        .unwrap()
                        .to_utc(),
                ),
                duration: 3615,
                project: Some("project1".to_string()),
                tags: vec!["tag1".to_string()],
            },
            5 => TimeEntry {
                description: "entry5".to_string(),
                start: DateTime::parse_from_rfc3339("2024-01-03T06:00:00+00:00")
                    .unwrap()
                    .to_utc(),
                stop: Some(
                    DateTime::parse_from_rfc3339("2024-01-03T07:00:20+00:00")
                        .unwrap()
                        .to_utc(),
                ),
                duration: 3620,
                project: Some("project1".to_string()),
                tags: vec!["tag1".to_string()],
            },
            6 => TimeEntry {
                description: "entry5".to_string(),
                start: DateTime::parse_from_rfc3339("2024-01-03T07:00:00+00:00")
                    .unwrap()
                    .to_utc(),
                stop: Some(
                    DateTime::parse_from_rfc3339("2024-01-03T08:00:25+00:00")
                        .unwrap()
                        .to_utc(),
                ),
                duration: 3625,
                project: Some("project2".to_string()),
                tags: vec!["tag1".to_string()],
            },
            7 => TimeEntry {
                description: "entry5".to_string(),
                start: DateTime::parse_from_rfc3339("2024-01-03T08:00:00+00:00")
                    .unwrap()
                    .to_utc(),
                stop: Some(
                    DateTime::parse_from_rfc3339("2024-01-03T09:00:30+00:00")
                        .unwrap()
                        .to_utc(),
                ),
                duration: 3630,
                project: Some("project1".to_string()),
                tags: vec!["tag2".to_string()],
            },
            // none stop
            8 => TimeEntry {
                description: "entry5".to_string(),
                start: DateTime::parse_from_rfc3339("2024-01-03T08:00:00+00:00")
                    .unwrap()
                    .to_utc(),
                stop: None,
                duration: -1,
                project: Some("project3".to_string()),
                tags: vec!["tag3".to_string()],
            },
            _ => panic!("Invalid pattern: {}", pattern),
        }
    }
}
