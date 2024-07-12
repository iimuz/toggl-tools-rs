use anyhow::{Context, Result};
use chrono::{DateTime, Local, NaiveDate, TimeZone, Timelike, Utc};
use log::info;

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
    ///
    /// # Arguments
    ///
    /// * `daily` - `daily`サブコマンドの引数
    pub async fn run(&self, daily: DailyArgs) -> Result<Vec<TimeEntry>> {
        // Localのタイムゾーンで00:00:00から始まる1日とする
        let date = daily.date.unwrap_or_else(Utc::now);
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
    use chrono::{DateTime, Local, TimeZone};
    use rstest::rstest;

    use super::DailyArgs;
    use super::DailyCommand;
    use crate::toggl::MockTogglRepository;

    #[tokio::test]
    async fn test_daily_command_no_date() {
        let args = DailyArgs { date: None };
        let mut toggl = MockTogglRepository::new();
        toggl
            .expect_read_time_entries()
            .times(1)
            .returning(|_, _| Ok(vec![]));

        let command = DailyCommand::new(&toggl);
        let result = command.run(args).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    #[rstest]
    #[case(Local::now())]
    #[case(Local.with_ymd_and_hms(2000, 1, 1, 0, 0, 0).unwrap())]
    async fn test_daily_command_with_date(#[case] date: DateTime<Local>) {
        let date_utc = date.to_utc();
        let daily = DailyArgs {
            date: Some(date_utc),
        };
        let mut toggl = MockTogglRepository::new();
        toggl
            .expect_read_time_entries()
            .times(1)
            .returning(|_, _| Ok(vec![]));

        let command = DailyCommand::new(&toggl);
        let result = command.run(daily).await;

        assert!(result.is_ok());
    }
}
