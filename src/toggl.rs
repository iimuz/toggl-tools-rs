use std::env;

use anyhow::{Context, Result};
use chrono::{DateTime, Local, TimeZone, Utc};
use reqwest::{header::CONTENT_TYPE, Client};
use serde::Deserialize;

use crate::time_entry::TimeEntry;

#[derive(Debug, Deserialize)]
pub struct TogglTimeEntry {
    at: String,
    billable: bool,
    pub description: String,
    duration: i64,
    duronly: bool,
    id: i64,
    pid: i64,
    project_id: Option<i64>,
    server_deleted_at: Option<String>,
    pub start: String,
    stop: Option<String>,
    tag_ids: Vec<i64>,
    tags: Vec<String>,
    task_id: Option<i64>,
    uid: i64,
    user_id: i64,
    wid: i64,
    workspace_id: i64,
}

pub struct TogglClient {
    client: Client,
    api_url: String,
    api_token: String,
}

impl TogglClient {
    pub fn new() -> Result<Self> {
        let api_token = env::var("TOGGL_API_TOKEN").context("TOGGL_API_TOKEN must be set")?;

        Ok(Self {
            client: Client::new(),
            api_url: "https://api.track.toggl.com/api/v9".to_string(),
            api_token: api_token.to_string(),
        })
    }

    pub async fn get_timer(&self, start_at: &DateTime<Utc>, end_at: &DateTime<Utc>) -> Result<Vec<TimeEntry>> {
        let toggl_time_entries = self
            .client
            .get(format!("{}/me/time_entries", self.api_url))
            .basic_auth(&self.api_token, Some("api_token"))
            .header(CONTENT_TYPE, "application/json")
            .query(&[("start_date", start_at.to_rfc3339()), ("end_date", end_at.to_rfc3339())])
            .send()
            .await
            .with_context(|| format!("Failed to send request to Toggl API at {}", self.api_url))?
            .error_for_status()
            .context("Request returned an error status")?
            .json::<Vec<TogglTimeEntry>>()
            .await
            .context("Failed to deserialize response")?;

        let time_entries = toggl_time_entries
            .into_iter()
            .map(|entry| TimeEntry {
                description: entry.description,
                start:DateTime::parse_from_rfc3339(&entry.start)
                    .unwrap()
                    .to_utc()
            })
            .collect();

        Ok(time_entries)
    }
}
