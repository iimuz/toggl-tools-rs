use std::path::PathBuf;
use std::{env, path::Path};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use dirs;

mod daily_command;
mod monthly_command;
mod time_entry;
mod toggl;

use daily_command::{daily_command, DailyCommand};
use fern::colors::{Color, ColoredLevelConfig};
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

fn determine_log_path() -> Result<PathBuf> {
    // 環境変数からログパスを取得（設定されていない場合はNone）
    let env_log_dir = env::var("TOOGLS_LOG_DIR").ok().map(PathBuf::from);
    let log_dir = env_log_dir.unwrap_or_else(|| {
        let app_name = env!("CARGO_PKG_NAME"); // Get the app name from cargo.toml
        let home_dir = dirs::home_dir().expect("Failed to determine home directory");
        let is_debug = cfg!(debug_assertions);
        let build_type = if is_debug { "debug" } else { "release" };
        let is_windows = cfg!(target_os = "windows");
        let os_type = if is_windows { "windows" } else { "unix" };
        match (build_type, os_type) {
            ("debug", _) => PathBuf::from("."),
            ("release", "windows") => home_dir.join(format!("AppData\\Local\\{}\\Logs", app_name)),
            ("release", _) => home_dir.join(format!(".local/share/{}/logs", app_name)),
            _ => unreachable!("Unsupported build type and OS type combination"),
        }
    });

    return Ok(log_dir);
}
fn init_logger(log_dir: &Path) {
    let colors = ColoredLevelConfig::new()
        .trace(Color::White)
        .info(Color::Green)
        .debug(Color::Cyan)
        .warn(Color::Yellow)
        .error(Color::Red);

    let console_config = fern::Dispatch::new()
        .level(log::LevelFilter::Trace)
        .format(move |out, message, record| {
            out.finish(format_args!(
                "[{}] {}:{} {} {}",
                colors.color(record.level()),
                record.file().unwrap(),
                record.line().unwrap(),
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                message
            ))
        })
        .chain(std::io::stderr());
    let path_app = log_dir.join("application.log");
    let application_config = fern::Dispatch::new()
        .level(log::LevelFilter::Info)
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}] {}:{} {} {}",
                record.level(),
                record.file().unwrap(),
                record.line().unwrap(),
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                message
            ))
        })
        .chain(fern::log_file(path_app).unwrap());

    let path_emergency = log_dir.join("emergency.log");
    let emergency_config = fern::Dispatch::new()
        .level(log::LevelFilter::Error)
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}] {}:{} {} {}",
                record.level(),
                record.file().unwrap(),
                record.line().unwrap(),
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                message
            ))
        })
        .chain(fern::log_file(path_emergency).unwrap());

    fern::Dispatch::new()
        .chain(console_config)
        .chain(application_config)
        .chain(emergency_config)
        .apply()
        .unwrap();
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let log_dir = determine_log_path().context("Failed to determine log path")?;
    init_logger(&log_dir);

    match args.subcommand {
        SubCommands::Daily(daily) => daily_command(daily).await?,
        SubCommands::Monthly(monthly) => monthly_command(monthly).await?,
    }

    Ok(())
}
