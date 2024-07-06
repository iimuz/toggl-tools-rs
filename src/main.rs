use std::env;

use anyhow::Result;
use clap::{Parser, Subcommand};
use env_logger;

mod daily_commnad;
mod time_entry;
mod toggl;

use daily_commnad::{DailyCommand, daily_command};

#[derive(Debug, Parser)]
#[clap(version, about)]
struct Args {
    #[clap(subcommand)]
    subcommand: SubCommands,
}

#[derive(Debug, Subcommand)]
enum SubCommands {
    Daily(DailyCommand),
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    env::set_var("RUST_LOG", "info");
    env_logger::init();

    match args.subcommand {
        SubCommands::Daily(daily) => daily_command(daily).await?,
    }

    Ok(())
}
