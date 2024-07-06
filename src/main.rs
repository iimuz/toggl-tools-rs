use std::env;

use anyhow::Result;
use clap::{Parser, Subcommand};
use env_logger;

mod daily_commnad;
mod monthly_command;
mod time_entry;
mod toggl;

use daily_commnad::{daily_command, DailyCommand};
use monthly_command::{monthly_command, MonthlyCommand};

/// time entryを取得するためのCLIアプリケーション。
///
/// # Examples
/// ```
/// $ cargo run -- daily
/// $ cargo run -- monthly
/// ```
#[derive(Debug, Parser)]
#[clap(version, about)]
struct Args {
    #[clap(subcommand)]
    subcommand: SubCommands,
}

/// サブコマンドを表す列挙型。
#[derive(Debug, Subcommand)]
enum SubCommands {
    Daily(DailyCommand),
    Monthly(MonthlyCommand),
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    env::set_var("RUST_LOG", "info");
    env_logger::init();

    match args.subcommand {
        SubCommands::Daily(daily) => daily_command(daily).await?,
        SubCommands::Monthly(monthly) => monthly_command(monthly).await?,
    }

    Ok(())
}
