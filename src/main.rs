use std::env;

use anyhow::{Context, Result};
use chrono::{DateTime, Local, NaiveDate, TimeZone, Timelike, Utc};
use clap::{Parser, Subcommand};
use env_logger;
use log::info;

mod time_entry;
mod toggl;

use time_entry::TimeEntry;
use toggl::TogglClient;

#[derive(Debug, Parser)]
#[clap(version, about)]
struct Args {
    #[clap(subcommand)]
    subcommand: SubCommands,
}

#[derive(Debug, Subcommand)]
enum SubCommands {
    Daily(Daily),
}

#[derive(Debug, clap::Args)]
struct Daily {
    #[clap(
        short = 'd',
        long = "date",
        help = "Sets a custom date in the format YYYY-MM-DD",
        parse(try_from_str = parse_date),
    )]
    date: Option<DateTime<Utc>>,
}

// 日付をパースして、LocalのタイムゾーンでのUNIX時間に変換する
//
// 例: "2021-01-01" -> 1609459200
fn parse_date(s: &str) -> Result<DateTime<Utc>> {
    // 時刻まで指定されている場合は、その値を直接利用する
    if s.contains("T") {
        let datetime = DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
            .context("Failed to parse date and time")?
            .with_timezone(&Local)
            .to_utc();

        return Ok(datetime);
    }

    // 時刻が指定されていない場合は、その日の0時0分0秒を利用する
    let naive_date = NaiveDate::parse_from_str(s, "%Y-%m-%d").context("Failed to parse date")?;
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

/// time etnryを表示する。
///
/// この関数は、`time_entries`の要素を時刻順にソートして、時刻と説明を表示する。
///
/// # Arguments
///
/// * `time_entries` - 表示する時刻エントリー
///
/// # Examples
///
/// ```
/// let time_entries = vec![
///    TimeEntry { start: 1609459200, description: "First entry".to_string() },
///    TimeEntry { start: 1609459300, description: "Second entry".to_string() },
/// ];
/// show_time_entries(&time_entries);
/// ```
///
/// この関数は以下のように表示する。
///
/// ```text
/// - 00:00 First entry
/// - 00:01 Second entry
/// ```
fn show_time_entries(time_entries: &Vec<TimeEntry>) {
    let mut sorted_entries = time_entries.clone();
    sorted_entries.sort_by_key(|entry| entry.start);

    for entry in sorted_entries {
        let start_str = entry.start.with_timezone(&Local).format("%H:%M");
        println!("- {} {}", start_str, entry.description)
    }
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
async fn daily_command(daily: Daily) -> Result<()> {
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
        .get_timer(&start_at.to_utc(), &end_at.to_utc())
        .await
        .context("Failed to retrieve time entries")?;

    info!("Time entries retrieved successfully.");
    show_time_entries(&time_entries);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    env::set_var("RUST_LOG", "info");
    env_logger::init();

    match args.subcommand {
        SubCommands::Daily(daily) => daily_command(daily).await?,
    }

    Ok(())
}
