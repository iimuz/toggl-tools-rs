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
    #[clap(short, long, parse(from_occurrences))]
    /// Sets the verbosity level for logging.
    /// Each occurrence increases the verbosity level.
    /// If not explicitly specified as an argument, it will be obtained from the `RUST_LOG` environment variable.
    /// If nothing is specified, it will default to the error level.
    verbose: u8,

    #[clap(subcommand)]
    subcommand: SubCommands,
}
#[derive(Debug, Subcommand)]
enum SubCommands {
    Daily(DailyCommand),
    Monthly(MonthlyCommand),
}

/// ログファイルのパスを決定する。
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

/// ロガーを初期化する。
fn init_logger(log_dir: &Path, log_level: &log::LevelFilter) {
    let colors = ColoredLevelConfig::new()
        .trace(Color::White)
        .info(Color::Green)
        .debug(Color::Cyan)
        .warn(Color::Yellow)
        .error(Color::Red);

    let console_config = fern::Dispatch::new()
        .level(*log_level)
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

    // 引数によるログレベルの指定がない場合は、環境変数から取得する。
    // ただし、環境変数もない場合は、error levelとする。
    let rust_log_level = match std::env::var("RUST_LOG")
        .unwrap_or_default()
        .to_lowercase()
        .as_str()
    {
        "trace" => Some(log::LevelFilter::Trace),
        "debug" => Some(log::LevelFilter::Debug),
        "info" => Some(log::LevelFilter::Info),
        "warn" => Some(log::LevelFilter::Warn),
        "error" => Some(log::LevelFilter::Error),
        "off" => Some(log::LevelFilter::Off),
        _ => None,
    };
    let log_level = match args.verbose {
        0 => rust_log_level.unwrap_or(log::LevelFilter::Error),
        1 => log::LevelFilter::Warn,
        2 => log::LevelFilter::Info,
        3 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    };
    let log_dir = determine_log_path().context("Failed to determine log path")?;
    init_logger(&log_dir, &log_level);

    match args.subcommand {
        SubCommands::Daily(daily) => daily_command(daily).await?,
        SubCommands::Monthly(monthly) => monthly_command(monthly).await?,
    }

    Ok(())
}
