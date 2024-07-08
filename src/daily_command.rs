use anyhow::{Context, Result};
use chrono::{DateTime, Local, NaiveDate, TimeZone, Timelike, Utc};
use log::info;

use crate::time_entry::TimeEntry;
use crate::toggl::TogglClient;

/// `daily`サブコマンドの引数を表す構造体。
#[derive(Debug, clap::Args)]
pub struct DailyCommand {
    #[clap(
        short = 'd',
        long = "date",
        help = "Sets a custom date in the format YYYY-MM-DD",
        parse(try_from_str = parse_date),
    )]
    date: Option<DateTime<Utc>>,
}

/// `daily`サブコマンドの処理を行う。
///
/// Localタイムゾーンで指定された日付の00:00:00から始まる1日のタイムエントリーを取得し、表示する。
/// 日付が指定されていない場合は、Localタイムゾーンで現在の日付を利用する。
///
/// # Arguments
///
/// * `daily` - `daily`サブコマンドの引数
///
/// # Examples
///
/// ```
/// let daily = Daily { date: None };
/// daily_command(daily).await.unwrap();
/// ```
pub async fn daily_command(daily: DailyCommand) -> Result<()> {
    // Localのタイムゾーンで00:00:00から始まる1日とする
    let date = daily.date.unwrap_or_else(|| Local::now().to_utc());
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

    let client = TogglClient::new().context("Failed to new toggl client")?;
    let time_entries = client
        .read_time_entries(&start_at.to_utc(), &end_at.to_utc())
        .await
        .context("Failed to retrieve time entries")?;

    info!("Time entries retrieved successfully.");
    show_time_entries(&time_entries);

    Ok(())
}

/// time entryを表示する。
fn show_time_entries(time_entries: &[TimeEntry]) {
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
        println!("- {} ~ {}: {}", start_str, end_str, entry.description)
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
