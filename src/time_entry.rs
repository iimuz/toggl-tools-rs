use chrono::{DateTime, Utc};

#[derive(Clone, Debug)]
pub struct TimeEntry {
	pub start: DateTime<Utc>,
	pub stop: Option<DateTime<Utc>>,
	pub description: String,
}
