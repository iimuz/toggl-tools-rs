use anyhow::{Context, Result};
use chrono::{Duration, Local, TimeZone, Timelike};
use env_logger;
use log::info;
use reqwest::{header::CONTENT_TYPE, Client};
use serde::Deserialize;
use std::env;

const TOGGL_API_URL: &str = "https://api.track.toggl.com/api/v9";

#[derive(Debug, Deserialize)]
struct TimeEntry {
    at: String,
    billable: bool,
    description: String,
    duration: i64,
    duronly: bool,
    id: i64,
    pid: i64,
    project_id: Option<i64>,
    server_deleted_at: Option<String>,
    start: String,
    stop: Option<String>,
    tag_ids: Vec<i64>,
    tags: Vec<String>,
    task_id: Option<i64>,
    uid: i64,
    user_id: i64,
    wid: i64,
    workspace_id: i64,
}

async fn get_timer(
    client: &Client,
    api_token: &str,
    start_at: i64,
    end_at: i64,
) -> Result<Vec<TimeEntry>> {
    let start_date = Local.timestamp_opt(start_at, 0).unwrap();
    let start_str = start_date.to_rfc3339();
    let end_date = Local.timestamp_opt(end_at, 0).unwrap();
    let end_str = end_date.to_rfc3339();

    let time_entry = client
        .get(format!("{}/me/time_entries", TOGGL_API_URL))
        .basic_auth(api_token, Some("api_token"))
        .header(CONTENT_TYPE, "application/json")
        .query(&[("start_date", start_str), ("end_date", end_str)])
        .send()
        .await
        .with_context(|| format!("Failed to send request to Toggl API at {}", TOGGL_API_URL))?
        .error_for_status()
        .context("Request returned an error status")?
        .json::<Vec<TimeEntry>>()
        .await
        .context("Failed to deserialize response")?;

    Ok(time_entry)
}

#[tokio::main]
async fn main() -> Result<()> {
    env::set_var("RUST_LOG", "info");
    env_logger::init();

    let api_token = env::var("TOGGL_API_TOKEN").context("TOGGL_API_TOKEN must be set")?;
    let client = Client::new();

    let start_at = Local::now()
        .with_hour(0)
        .context("Failed to set hour")?
        .with_minute(0)
        .context("Failed to set minute")?
        .with_second(0)
        .context("Failed to set second")?
        - Duration::days(5);
    let end_at = start_at + Duration::days(1);
    let time_entry = get_timer(
        &client,
        &api_token,
        start_at.timestamp(),
        end_at.timestamp(),
    )
    .await
    .context("Failed to retrieve time entries")?;

    info!("Time entries retrieved successfully.");
    println!("{:#?}", time_entry);

    Ok(())
}
