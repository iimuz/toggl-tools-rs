use std::{collections::HashMap, env};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use log::info;
use reqwest::{header::CONTENT_TYPE, Client};
use serde::Deserialize;

use crate::time_entry::TimeEntry;

/// Toggl APIのレスポンスをデシリアライズするための構造体。
#[derive(Debug, Deserialize)]
struct TogglTimeEntry {
    description: String,
    project_id: Option<i64>,
    start: String,
    stop: Option<String>,
    duration: i64,
    tags: Vec<String>,
}

/// Toggl APIのプロジェクト情報をデシリアライズするための構造体。
#[derive(Debug, Deserialize)]
struct TogglProject {
    id: i64,
    name: String,
}

/// Toggl APIと通信するためのクライアント。
///
/// # Examples
///
/// ```
/// let client = TogglClient::new().unwrap();
/// let time_entries = client.get_timer(&start_at, &end_at).await.unwrap();
/// ```
pub struct TogglClient {
    client: Client,
    api_url: String,
    api_token: String,
}

impl TogglClient {
    /// 新しい`TogglClient`を返す。
    ///
    /// 環境変数`TOGGL_API_TOKEN`が設定されていない場合はエラーを返す。
    ///
    /// # Examples
    ///
    /// ```
    /// let client = TogglClient::new().unwrap();
    /// ```
    pub fn new() -> Result<Self> {
        let api_token = env::var("TOGGL_API_TOKEN").context("TOGGL_API_TOKEN must be set")?;

        Ok(Self {
            client: Client::new(),
            api_url: "https://api.track.toggl.com/api/v9".to_string(),
            api_token: api_token.to_string(),
        })
    }

    /// 指定された日付のタイムエントリーを取得する。
    ///
    /// # Arguments
    ///
    /// * `start_at` - 取得するタイムエントリーの開始日時
    /// * `end_at` - 取得するタイムエントリーの終了日時
    ///
    /// # Examples
    ///
    /// ```
    /// let time_entries = client.get_timer(&start_at, &end_at).await.unwrap();
    /// ```
    pub async fn read_time_entries(
        &self,
        start_at: &DateTime<Utc>,
        end_at: &DateTime<Utc>,
    ) -> Result<Vec<TimeEntry>> {
        let toggl_time_entries = self
            .client
            .get(format!("{}/me/time_entries", self.api_url))
            .basic_auth(&self.api_token, Some("api_token"))
            .header(CONTENT_TYPE, "application/json")
            .query(&[
                ("start_date", start_at.to_rfc3339()),
                ("end_date", end_at.to_rfc3339()),
            ])
            .send()
            .await
            .with_context(|| format!("Failed to send request to Toggl API at {}", self.api_url))?
            .error_for_status()
            .context("Request returned an error status")?
            .json::<Vec<TogglTimeEntry>>()
            .await
            .context("Failed to deserialize response")?;
        info!("length or tme entries: {}", toggl_time_entries.len());
        let toggl_projects = self
            .read_projects()
            .await
            .context("Failed to get project list from toggl")?;
        let toggl_projects_map: HashMap<i64, TogglProject> = toggl_projects
            .into_iter()
            .map(|project| (project.id, project))
            .collect();

        let time_entries = toggl_time_entries
            .into_iter()
            .map(|entry| {
                let start = DateTime::parse_from_rfc3339(&entry.start).unwrap().to_utc();
                let stop = entry
                    .stop
                    .map(|stop| DateTime::parse_from_rfc3339(&stop).unwrap().to_utc());
                let project = match entry.project_id {
                    Some(project_id) => toggl_projects_map
                        .get(&project_id)
                        .map(|project| project.name.clone()),
                    None => None,
                };

                TimeEntry {
                    start,
                    stop,
                    duration: entry.duration,
                    description: entry.description,
                    project,
                    tags: entry.tags,
                }
            })
            .collect();

        Ok(time_entries)
    }

    /// プロジェクト情報を取得する。
    async fn read_projects(&self) -> Result<Vec<TogglProject>> {
        let projects = self
            .client
            .get(format!("{}/me/projects", self.api_url))
            .basic_auth(&self.api_token, Some("api_token"))
            .header(CONTENT_TYPE, "application/json")
            .send()
            .await
            .with_context(|| format!("Failed to send request to Toggl API at {}", self.api_url))?
            .error_for_status()
            .context("Request returned an error status")?
            .json::<Vec<TogglProject>>()
            .await
            .context("Failed to deserialize response")?;

        Ok(projects)
    }
}
