use anyhow::{Context, Result};
use chrono::{DateTime, Local, NaiveDate, TimeZone, Timelike, Utc};
use log::info;

use crate::datetime::now;
use crate::time_entry::TimeEntry;
use crate::toggl::TogglRepository;

/// 日毎の情報を出力するためのサブコマンド。
#[derive(Debug, clap::Args)]
pub struct DailyArgs {
    #[clap(
        short = 'd',
        long = "date",
        help = "Sets a custom date in the format YYYY-MM-DD",
        parse(try_from_str = parse_date),
    )]
    date: Option<DateTime<Utc>>,
}

pub struct DailyCommand<'a, T: TogglRepository> {
    toggl_client: &'a T,
}

impl<'a, T: TogglRepository> DailyCommand<'a, T> {
    /// 新しい`DailyCommand`を返す。
    ///
    /// # Arguments
    /// * `toggl_client` - Toggl APIと通信するためのリポジトリ
    pub fn new(toggl_client: &'a T) -> Self {
        Self { toggl_client }
    }

    /// `daily`サブコマンドの処理を行う。
    ///
    /// Localタイムゾーンで指定された日付の00:00:00から始まる1日のタイムエントリーを取得し、表示する。
    /// 日付が指定されていない場合は、Localタイムゾーンで現在の日付を利用する。
    pub async fn run(&self, daily: DailyArgs) -> Result<Vec<TimeEntry>> {
        // Localのタイムゾーンで00:00:00から始まる1日とする
        let date = daily.date.unwrap_or_else(now);
        let local_date = date.with_timezone(&Local);
        let start_at = local_date
            .with_hour(0)
            .context("Failed to set hour")?
            .with_minute(0)
            .context("Failed to set minute")?
            .with_second(0)
            .context("Failed to set second")?;
        let end_at = start_at + chrono::Duration::days(1);

        info!("Start at: {}, End at: {}", start_at, end_at);
        let time_entries = self
            .toggl_client
            .read_time_entries(&start_at.to_utc(), &end_at.to_utc())
            .await
            .context("Failed to retrieve time entries")?;
        info!("Time entries retrieved successfully.");

        Ok(time_entries)
    }
}

/// 日付をパースする。
fn parse_date(s: &str) -> Result<DateTime<Utc>> {
    let naive_date = NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .with_context(|| format!("Failed to parse date: {}", s))?;
    let naive_datetime = naive_date
        .and_hms_opt(0, 0, 0)
        .context("Failed to set hour, minute, and second")?;
    let datetime = Local
        .from_local_datetime(&naive_datetime)
        .single()
        .context("Failed to convert to DateTime<Local>")?
        .to_utc();

    Ok(datetime)
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Timelike, Utc};
    use mockall::predicate;
    use rstest::rstest;

    use super::parse_date;
    use super::DailyArgs;
    use super::DailyCommand;
    use crate::datetime::mock_datetime;
    use crate::time_entry::TimeEntry;
    use crate::toggl::MockTogglRepository;

    #[tokio::test]
    #[rstest]
    #[case::none_date_to_now(None)]
    #[case::specific_date(Some(DateTime::parse_from_rfc3339("2024-01-01T00:00:00+00:00").unwrap().to_utc()))]
    async fn test_daily_command_no_date(#[case] date: Option<DateTime<Utc>>) {
        let args = DailyArgs { date };
        let mut toggl = MockTogglRepository::new();

        let now = date.unwrap_or(Utc::now());
        let today = now
            .with_timezone(&Local)
            .with_hour(0)
            .unwrap()
            .with_minute(0)
            .unwrap()
            .with_second(0)
            .unwrap();
        let tomorrow = today + chrono::Duration::days(1);
        mock_datetime::set_mock_time(now);

        let entries = vec![TimeEntry {
            description: "test 1".to_string(),
            start: today.with_hour(3).unwrap().to_utc(),
            stop: Some(today.with_hour(4).unwrap().to_utc()),
            duration: 3600,
            project: None,
            tags: vec![],
        }];
        let expect_entries = entries.clone();
        toggl
            .expect_read_time_entries()
            .with(
                predicate::eq(today.to_utc()),
                predicate::eq(tomorrow.to_utc()),
            )
            .times(1)
            .returning(move |_, _| Ok(entries.clone()));

        let command = DailyCommand::new(&toggl);
        let result = command.run(args).await;

        assert!(result.is_ok());
        assert_eq!(expect_entries, result.unwrap());
    }

    /// time entriesの取得に失敗した場合にエラーとなることを確認する。
    #[tokio::test]
    async fn test_error_daily_command_get_time_entries() {
        let daily = DailyArgs { date: None };
        let mut toggl = MockTogglRepository::new();
        toggl
            .expect_read_time_entries()
            .times(1)
            .returning(|_, _| Err(anyhow::anyhow!("Test error")));

        let command = DailyCommand::new(&toggl);
        let result = command.run(daily).await;

        assert!(result.is_err());
    }

    /// 正常に日付をパースできることを確認する。
    #[test]
    fn test_parse_date_valid_date() {
        let date_str = "2022-12-31";
        let expected_date = Local
            .from_local_datetime(
                &NaiveDateTime::parse_from_str("2022-12-31T00:00:00", "%Y-%m-%dT%H:%M:%S").unwrap(),
            )
            .unwrap()
            .to_utc();

        let result = parse_date(date_str);

        assert!(result.is_ok());
        assert_eq!(expected_date, result.unwrap());
    }

    /// 入力日付が間違っている場合にエラーを返すことを確認する。
    #[rstest]
    #[test]
    #[case::invalid_year("20xx-01-01")]
    #[case::invalid_month("2024-13-01")]
    #[case::invalid_day("2024-02-30")]
    #[case::invalid_format("2024/01/01")]
    #[case::empty_string("")]
    fn test_parse_date_invalid_date(#[case] date_str: &str) {
        let result = parse_date(date_str);

        assert!(result.is_err());
    }
}
