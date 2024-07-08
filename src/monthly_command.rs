use std::collections::HashMap;

use anyhow::{Context, Ok, Result};
use chrono::{DateTime, Datelike, Local, NaiveDate, TimeZone, Timelike, Utc};
use log::info;

use crate::time_entry::TimeEntry;
use crate::toggl::TogglClient;

/// `monthly`サブコマンドの引数を表す構造体。
#[derive(Debug, clap::Args)]
pub struct MonthlyCommand {
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

/// `monthly`サブコマンドの処理を行う。
///
/// Localタイムゾーンで指定された月のtime entryでタグごとの集計結果を日毎に出力する。
/// 日付が指定されていない場合は、Localタイムゾーンで現在の月を利用する。
///
/// # Arguments
///
/// * `monthly` - `monthly`サブコマンドの引数
///
/// # Examples
///
/// ```
/// let daily = Daily { date: None };
/// monthly_command(daily).await.unwrap();
/// ```
pub async fn monthly_command(monthly: MonthlyCommand) -> Result<()> {
    // Localのタイムゾーンで00:00:00から始まる1日とする
    let date = monthly.month.unwrap_or_else(|| Local::now().to_utc());
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
    let end_at = start_at
        .with_month(start_at.month() + 1)
        .context("Failed to set month")?;
    info!("Start at: {}, End at: {}", start_at, end_at);

    let client = TogglClient::new().context("Failed to new toggl client")?;
    let time_entries = client
        .read_time_entries(&start_at.to_utc(), &end_at.to_utc())
        .await
        .context("Failed to retrieve time entries")?;
    info!("Time entries retrieved successfully.");

    if monthly.daily {
        let daily_time_entries: HashMap<NaiveDate, Vec<TimeEntry>> =
            time_entries.iter().fold(HashMap::new(), |mut acc, entry| {
                let start = entry.start.with_timezone(&Local).date_naive();
                acc.entry(start).or_default().push(entry.clone());
                acc
            });
        let mut sorted_time_entries = daily_time_entries
            .iter()
            .map(|(date, entries)| (*date, entries.clone()))
            .collect::<Vec<_>>();
        sorted_time_entries.sort_by_key(|(date, _)| *date);
        sorted_time_entries.iter().try_for_each(|(date, entries)| {
            println!("## {}", date);
            let daily_duration = calc_project_tag_duration(entries).with_context(|| {
                format!(
                    "Failed to calculate project tag duration for date: {}",
                    date
                )
            })?;
            show_durations(&daily_duration);
            Ok(())
        })?;
    } else {
        let project_tag_duration = calc_project_tag_duration(&time_entries)
            .context("Failed to calculate project tag duration")?;
        show_durations(&project_tag_duration);
    }

    Ok(())
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
        .single()
        .context("Failed to convert to DateTime<Local>")?
        .to_utc();

    Ok(datetime)
}

/// プロジェクトごと、かつタグごとの集計結果を計算する。
///
/// 終了していないtime entryは集計対象外とする。
fn calc_project_tag_duration(
    time_entries: &[TimeEntry],
) -> Result<HashMap<String, HashMap<String, i64>>> {
    let project_tag_duration: HashMap<String, HashMap<String, i64>> =
        time_entries
            .iter()
            .fold(HashMap::new(), |mut accumurate, entry| {
                if entry.stop.is_none() {
                    return accumurate;
                }

                let key = entry.project.clone().unwrap_or_default();
                let project_entry = accumurate.entry(key).or_default();
                entry.tags.iter().for_each(|tag| {
                    *project_entry.entry(tag.clone()).or_insert(0) += entry.duration;
                });
                accumurate
            });

    Ok(project_tag_duration)
}

/// project, tagごとの集計結果を表示する。
///
/// 表示は時間単位で行う。
fn show_durations(durations: &HashMap<String, HashMap<String, i64>>) {
    durations.iter().for_each(|(project, tags)| {
        println!("- {}", project);
        tags.iter().for_each(|(tag, duration)| {
            let duration_hours = *duration as f64 / 3600.0;
            println!("  - {}: {:.2}", tag, duration_hours);
        });
    });
}
