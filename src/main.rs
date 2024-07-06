use std::env;

use anyhow::{Context, Result};
use chrono::{DateTime, Local, NaiveDate, TimeZone, Timelike};
use clap::Parser;
use env_logger;
use log::info;

mod time_entry;
mod toggl;

use time_entry::TimeEntry;
use toggl::TogglClient;

#[derive(Parser)]
#[clap(version, about)]
struct Args {
    #[clap(
        short = 's',
        long = "start_at",
        help = "Sets a custom start time in the format YYYY-MM-DD",
        parse(try_from_str = parse_date),
    )]
    start_at: Option<i64>,

    #[clap(
        short = 'e',
        long = "end_at",
        help = "Sets a custom end time in the format YYYY-MM-DD",
        parse(try_from_str = parse_date)
    )]
    end_at: Option<i64>,
}

// 日付をパースして、LocalのタイムゾーンでのUNIX時間に変換する
//
// 例: "2021-01-01" -> 1609459200
fn parse_date(s: &str) -> Result<i64> {
    // 時刻まで指定されている場合は、その値を直接利用する
    if s.contains("T") {
        let datetime = DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
            .context("Failed to parse date and time")?
            .with_timezone(&Local);

        return Ok(datetime.timestamp());
    }

    // 時刻が指定されていない場合は、その日の0時0分0秒を利用する
    let naive_date = NaiveDate::parse_from_str(s, "%Y-%m-%d").context("Failed to parse date")?;
    let naive_datetime = naive_date
        .and_hms_opt(0, 0, 0)
        .context("Failed to set hour, minute, and second")?;
    let datetime = Local
        .from_local_datetime(&naive_datetime)
        .single()
        .context("Failed to convert to DateTime<Local>")?;

    Ok(datetime.timestamp())
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
        let start_str = Local.timestamp_opt(entry.start, 0).unwrap().format("%H:%M");
        println!("- {} {}", start_str, entry.description)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    env::set_var("RUST_LOG", "info");
    env_logger::init();

    // 時刻設定がされていない場合に今日の日付を利用する
    let now = Local::now();
    let today_start = now
        .with_hour(0)
        .context("Failed to set hour")?
        .with_minute(0)
        .context("Failed to set minute")?
        .with_second(0)
        .context("Failed to set second")?;
    let start_at = args.start_at.unwrap_or_else(|| today_start.timestamp());
    let end_at = args.end_at.unwrap_or_else(|| now.timestamp());
    info!("Start at: {}", Local.timestamp_opt(start_at, 0).unwrap());
    info!("End at: {}", Local.timestamp_opt(end_at, 0).unwrap());

    let client = TogglClient::new().context("Failed to new toggl client")?;
    let time_entries = client
        .get_timer(start_at, end_at)
        .await
        .context("Failed to retrieve time entries")?;

    info!("Time entries retrieved successfully.");
    show_time_entries(&time_entries);

    Ok(())
}
