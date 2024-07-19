use std::collections::HashMap;

use anyhow::{Context, Ok, Result};
use chrono::{DateTime, Datelike, Local, NaiveDate, TimeZone, Timelike, Utc};
use log::info;

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
        let date = monthly.month.unwrap_or_else(|| Local::now().to_utc());
        let (start_at, end_at) =
            calc_start_and_end_date(date).context("Failed to calculate start and end date")?;

        info!("Start at: {}, End at: {}", start_at, end_at);
        let time_entries = self
            .toggl_client
            .read_time_entries(&start_at, &end_at)
            .await
            .context("Failed to retrieve time entries")?;
        info!("Time entries retrieved successfully.");

        let durations = calc_project_tag_duration(&time_entries)
            .context("Failed to calculate project tag duration")?;

        Ok(durations)
    }

    // monthly sub commandで日毎の集計情報を返す。
    //
    // 集計結果はLocal dateで日付ごとになる。
    // 集計できない日付があった場合は、その日のみエラーを返す。
    pub async fn run_daily_duration(
        &self,
        monthly: MonthlyArgs,
    ) -> Result<Vec<(DateTime<Utc>, Result<ProjectDurations>)>> {
        // Localのタイムゾーンで00:00:00から始まる1日とする
        let date = monthly.month.unwrap_or_else(|| Local::now().to_utc());
        let (start_at, end_at) =
            calc_start_and_end_date(date).context("Failed to calculate start and end date")?;

        info!("Start at: {}, End at: {}", start_at, end_at);
        let time_entries = self
            .toggl_client
            .read_time_entries(&start_at, &end_at)
            .await
            .context("Failed to retrieve time entries")?;
        info!("Time entries retrieved successfully.");

        let daily_time_entries: HashMap<DateTime<Utc>, Vec<TimeEntry>> =
            time_entries.iter().fold(HashMap::new(), |mut acc, entry| {
                let start = entry.start.with_timezone(&Local).to_utc();
                acc.entry(start).or_default().push(entry.clone());
                acc
            });
        let durations = daily_time_entries
            .iter()
            .map(|(date, entries)| {
                let result = calc_project_tag_duration(entries);
                (*date, result)
            })
            .collect::<Vec<_>>();

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
fn calc_project_tag_duration(time_entries: &[TimeEntry]) -> Result<ProjectDurations> {
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

    Ok(project_tag_duration)
}

// 指定した日時のlocalの日時を含む月の開始日時と終了日時を返す。
fn calc_start_and_end_date(date: DateTime<Utc>) -> Result<(DateTime<Utc>, DateTime<Utc>)> {
    let local_date = date.with_timezone(&Local);
    let start_at = local_date
        .with_day0(0)
        .expect("Failed to set day")
        .with_hour(0)
        .expect("Failed to set hour")
        .with_minute(0)
        .expect("Failed to set minute")
        .with_second(0)
        .expect("Failed to set second");
    let end_at = start_at
        .with_month(start_at.month() + 1)
        .expect("Failed to set month");

    Ok((start_at.to_utc(), end_at.to_utc()))
}
