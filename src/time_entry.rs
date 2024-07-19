use std::collections::HashMap;

use chrono::{DateTime, Utc};

pub type ProjectName = String;
pub type TagName = String;
pub type TagDurations = HashMap<TagName, i64>;
pub type ProjectDurations = HashMap<ProjectName, TagDurations>;

#[derive(Clone, PartialEq, Debug)]
pub struct TimeEntry {
    pub start: DateTime<Utc>,
    pub stop: Option<DateTime<Utc>>,
    pub duration: i64,
    pub description: String,

    pub project: Option<ProjectName>,
    pub tags: Vec<TagName>,
}
