use std::env;
use chrono::{Duration, Local, TimeZone, Timelike};
use reqwest::{header::CONTENT_TYPE, Client};
use serde::Deserialize;

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

async fn start_timer(client: &Client, api_token: &str, start_at: i64, end_at: i64) -> Result<u32, reqwest::Error> {
    let start_date = Local.timestamp_opt(*start_at,0).unwrap();
    let start_str = start_date.to_rfc3339();
    let end_date = Local.timestamp_opt(*end_at, 0).unwrap();
    let end_str = end_date.to_rfc3339();

    let time_entry = client.get(format!("{}/me/time_entries", TOGGL_API_URL))
        .basic_auth(api_token, Some("api_token"))
        .header(CONTENT_TYPE, "application/json")
        .query(&[("start_date", start_str), ("end_date", end_str)])
        .send()
        .await?
        .error_for_status()?
        .json::<Vec<TimeEntry>>()
        .await?;

    println!("{:#?}", time_entry);

    Ok(3)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let api_token = env::var("TOGGL_API_TOKEN").expect("TOGGL_API_TOKEN must be set");
	let client = Client::new();

    let start_at = Local::now().with_hour(0).unwrap().with_minute(0).unwrap().with_second(0).unwrap() - Duration::days(5);
    let end_at = start_at + Duration::days(1);
    let id = start_timer(&client, &api_token, start_at.timestamp(), end_at.timestamp()).await?;
	println!("Started timer with id: {}", id);

	Ok(())
}
