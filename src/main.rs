mod toggl;
use toggl::TogglClient;

use anyhow::{Context, Result};
use chrono::{Duration, Local, Timelike};
use env_logger;
use log::info;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    env::set_var("RUST_LOG", "info");
    env_logger::init();

    let client = TogglClient::new().context("Failed to new toggl client")?;
    let start_at = Local::now()
        .with_hour(0)
        .context("Failed to set hour")?
        .with_minute(0)
        .context("Failed to set minute")?
        .with_second(0)
        .context("Failed to set second")?
        - Duration::days(5);
    let end_at = start_at + Duration::days(1);
    let time_entry = client
        .get_timer(start_at.timestamp(), end_at.timestamp())
        .await
        .context("Failed to retrieve time entries")?;

    info!("Time entries retrieved successfully.");
    println!("{:#?}", time_entry);

    Ok(())
}
