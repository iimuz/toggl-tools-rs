use std::{collections::HashMap, env};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
#[cfg(test)]
use mockall::automock;
use reqwest::{header::CONTENT_TYPE, Client};
use serde::Deserialize;
#[cfg(test)]
use serde::Serialize;

use crate::time_entry::TimeEntry;

#[cfg_attr(test, automock)]
/// Toggl APIと通信するためのリポジトリ。
pub trait TogglRepository {
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
    async fn read_time_entries(
        &self,
        start_at: &DateTime<Utc>,
        end_at: &DateTime<Utc>,
    ) -> Result<Vec<TimeEntry>>;
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
    pub fn new() -> Result<Self> {
        let api_token = env::var("TOGGL_API_TOKEN").context("TOGGL_API_TOKEN must be set")?;

        Ok(Self {
            client: Client::new(),
            api_url: "https://api.track.toggl.com/api/v9".to_string(),
            api_token: api_token.to_string(),
        })
    }
}

impl TogglRepository for TogglClient {
    async fn read_time_entries(
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
}

/// Toggl APIのレスポンスをデシリアライズするための構造体。
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
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
#[cfg_attr(test, derive(Serialize))]
struct TogglProject {
    id: i64,
    name: String,
}

impl TogglClient {
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

#[cfg(test)]
mod tests {
    use std::vec;

    use anyhow::Result;
    use base64::prelude::*;
    use chrono::DateTime;
    use mockito::Server;
    use rstest::rstest;

    use super::TogglClient;
    use super::TogglProject;
    use super::TogglRepository;
    use super::TogglTimeEntry;
    use crate::time_entry::TimeEntry;

    impl TogglClient {
        fn new_test(url: &str, api_token: &str) -> Result<Self> {
            Ok(Self {
                client: reqwest::Client::new(),
                api_url: url.to_string(),
                api_token: api_token.to_string(),
            })
        }
    }

    // 正常系のテスト
    #[tokio::test]
    #[rstest]
    #[case::normal(&[dummy_time_entry(1)], &[dummy_projects(0)])]
    #[case::no_entry(&[], &[dummy_projects(0)])]
    #[case::no_projects(&[dummy_time_entry(1)], &[])]
    #[case::no_entry_no_projects(&[], &[])]
    async fn test_read_time_entries(
        #[case] time_entries: &[TogglTimeEntry],
        #[case] projects: &[TogglProject],
    ) {
        // テストデータの作成
        let api_token = "test";
        let start_at = DateTime::parse_from_rfc3339("2024-01-02T00:00:00+09:00")
            .unwrap()
            .to_utc();
        let end_at = DateTime::parse_from_rfc3339("2024-01-03T00:00:00+09:00")
            .unwrap()
            .to_utc();
        let expected_entries: Vec<TimeEntry> = time_entries
            .iter()
            .map(|entry| to_time_entry(entry, projects))
            .collect();

        // モックサーバーの起動
        let mut server = Server::new_async().await;
        let url = server.url();
        let authorization = format!(
            "Basic {}",
            BASE64_STANDARD.encode(format!("{}:api_token", api_token))
        );
        let m1 = server
            .mock("GET", "/me/time_entries")
            .match_header("Authorization", authorization.as_str())
            .match_header("content-type", "application/json")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("start_date".into(), start_at.to_rfc3339()),
                mockito::Matcher::UrlEncoded("end_date".into(), end_at.to_rfc3339()),
            ]))
            .with_status(200)
            .with_body(serde_json::to_string(&time_entries).unwrap())
            .create_async()
            .await;
        let m2 = server
            .mock("GET", "/me/projects")
            .match_header("Authorization", authorization.as_str())
            .match_header("content-type", "application/json")
            .with_status(200)
            .with_body(serde_json::to_string(&projects).unwrap())
            .create_async()
            .await;

        // テストの実行
        let client = TogglClient::new_test(&url, api_token).unwrap();
        let time_entries = client.read_time_entries(&start_at, &end_at).await.unwrap();
        m1.assert_async().await;
        m2.assert_async().await;
        assert_eq!(expected_entries, time_entries);
    }

    // time entryの取得に失敗する場合のテスト
    #[tokio::test]
    #[rstest]
    #[case::code400(400)]
    #[case::code403(403)]
    #[case::code403(404)]
    #[case::code500(500)]
    async fn test_read_time_entries_error_toggl_time_entries(#[case] error_code: usize) {
        // テストデータの作成
        let api_token = "test";
        let start_at = DateTime::parse_from_rfc3339("2024-01-02T00:00:00+09:00")
            .unwrap()
            .to_utc();
        let end_at = DateTime::parse_from_rfc3339("2024-01-03T00:00:00+09:00")
            .unwrap()
            .to_utc();

        // モックサーバーの起動
        let mut server = Server::new_async().await;
        let url = server.url();
        let authorization = format!(
            "Basic {}",
            BASE64_STANDARD.encode(format!("{}:api_token", api_token))
        );
        let m1 = server
            .mock("GET", "/me/time_entries")
            .match_header("Authorization", authorization.as_str())
            .match_header("content-type", "application/json")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("start_date".into(), start_at.to_rfc3339()),
                mockito::Matcher::UrlEncoded("end_date".into(), end_at.to_rfc3339()),
            ]))
            .with_status(error_code)
            .create_async()
            .await;

        // テストの実行
        let client = TogglClient::new_test(&url, api_token).unwrap();
        let result = client.read_time_entries(&start_at, &end_at).await;
        m1.assert_async().await;
        assert!(result.is_err());
    }

    // projectsの取得に失敗する場合のテスト
    #[tokio::test]
    #[rstest]
    #[case::code400(400)]
    #[case::code403(403)]
    #[case::code403(404)]
    #[case::code500(500)]
    async fn test_read_time_entries_error_toggl_projects(#[case] error_code: usize) {
        // テストデータの作成
        let api_token = "test";
        let start_at = DateTime::parse_from_rfc3339("2024-01-02T00:00:00+09:00")
            .unwrap()
            .to_utc();
        let end_at = DateTime::parse_from_rfc3339("2024-01-03T00:00:00+09:00")
            .unwrap()
            .to_utc();
        let time_entries = vec![dummy_time_entry(1)];

        // モックサーバーの起動
        let mut server = Server::new_async().await;
        let url = server.url();
        let authorization = format!(
            "Basic {}",
            BASE64_STANDARD.encode(format!("{}:api_token", api_token))
        );
        let m1 = server
            .mock("GET", "/me/time_entries")
            .match_header("Authorization", authorization.as_str())
            .match_header("content-type", "application/json")
            .match_query(mockito::Matcher::AllOf(vec![
                mockito::Matcher::UrlEncoded("start_date".into(), start_at.to_rfc3339()),
                mockito::Matcher::UrlEncoded("end_date".into(), end_at.to_rfc3339()),
            ]))
            .with_status(200)
            .with_body(serde_json::to_string(&time_entries).unwrap())
            .create_async()
            .await;
        let m2 = server
            .mock("GET", "/me/projects")
            .match_header("Authorization", authorization.as_str())
            .match_header("content-type", "application/json")
            .with_status(error_code)
            .create_async()
            .await;

        // テストの実行
        let client = TogglClient::new_test(&url, api_token).unwrap();
        let result = client.read_time_entries(&start_at, &end_at).await;
        m1.assert_async().await;
        m2.assert_async().await;
        assert!(result.is_err());
    }

    // ダミータイムエントリを作成する
    fn dummy_time_entry(pattern: u8) -> TogglTimeEntry {
        match pattern {
            // 基本的な設定
            1 => TogglTimeEntry {
                description: "entry 1".to_string(),
                project_id: Some(0),
                start: "2024-01-02T01:02:03+09:00".to_string(),
                stop: Some("2024-01-02T01:02:04+09:00".to_string()),
                duration: 1,
                tags: vec!["tag 1".to_string()],
            },
            _ => panic!("Invalid pattern: {}", pattern),
        }
    }

    // ダミープロジェクトを作成する
    fn dummy_projects(pattern: u8) -> TogglProject {
        match pattern {
            0 => TogglProject {
                id: 0,
                name: "project 1".to_string(),
            },
            _ => panic!("Invalid pattern: {}", pattern),
        }
    }

    // Toggl Time EntryからTime Entryに変換する。
    //
    // 期待値の計算のため、計算時間を考慮せず、naiveな実装としている
    fn to_time_entry(entry: &TogglTimeEntry, projects: &[TogglProject]) -> TimeEntry {
        let start = DateTime::parse_from_rfc3339(&entry.start).unwrap().to_utc();
        let stop = entry
            .stop
            .clone()
            .map(|stop| DateTime::parse_from_rfc3339(&stop).unwrap().to_utc());
        let project = entry
            .project_id
            .map(|id| {
                projects
                    .iter()
                    .find(|project| project.id == id)
                    .map(|project| project.name.clone())
            })
            .unwrap();

        crate::time_entry::TimeEntry {
            start,
            stop,
            duration: entry.duration,
            description: entry.description.clone(),
            project,
            tags: entry.tags.clone(),
        }
    }
}
