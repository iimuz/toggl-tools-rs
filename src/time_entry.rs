use chrono::{DateTime, Utc};

#[derive(Clone, PartialEq, Debug)]
pub struct TimeEntry {
    pub start: DateTime<Utc>,
    pub stop: Option<DateTime<Utc>>,
    pub duration: i64,
    pub description: String,

    pub project: Option<String>,
    pub tags: Vec<String>,
}
