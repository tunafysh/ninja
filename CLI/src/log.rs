use log::LevelFilter;
use std::{path::Path, sync::OnceLock};
use tracing_appender::{non_blocking::WorkerGuard, rolling};
use tracing_subscriber::{EnvFilter, Layer, fmt, layer::SubscriberExt};

static FILE_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

pub fn setup_logger(level: LevelFilter) -> Result<(), Box<dyn std::error::Error>> {
    let log_path = match std::env::consts::OS {
        "linux" => format!(
            "{}{}",
            std::env::var("HOME").expect("Failed to get environment variable"),
            "/.local/share/com.tunafysh.ninja/logs/shurikenctl.log"
        ),
        "macos" => format!(
            "{}{}",
            std::env::var("HOME").expect("Failed to get environment variable"),
            "/Library/Application Support/com.tunafysh.ninja/logs/shurikenctl.log"
        ),
        "windows" => format!(
            "{}{}",
            std::env::var("LOCALAPPDATA").expect("Failed to get environment variable"),
            "\\com.tunafysh.ninja\\logs\\shurikenctl.log"
        ),
        _ => "logs/shurikenctl.log".to_string(),
    };

    let log_dir = Path::new(&log_path)
        .parent()
        .ok_or("log path is missing a parent directory")?;
    std::fs::create_dir_all(log_dir)?;

    let file_appender = rolling::daily(log_dir, "shurikenctl.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let _ = FILE_GUARD.set(guard);

    let level_filter = match level {
        LevelFilter::Off => "off".to_string(),
        LevelFilter::Error => "error".to_string(),
        LevelFilter::Warn => "warn".to_string(),
        LevelFilter::Info => "info".to_string(),
        LevelFilter::Debug => "debug".to_string(),
        LevelFilter::Trace => "trace".to_string(),
    };

    let filter = EnvFilter::new(level_filter.clone()).add_directive("rustpython=off".parse()?);

    tracing_log::LogTracer::builder()
        .with_max_level(level)
        .init()?;

    let stdout_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_span_events(fmt::format::FmtSpan::NEW | fmt::format::FmtSpan::CLOSE)
        .with_ansi(true)
        .with_file(true)
        .with_line_number(true)
        .with_filter(filter.clone());

    let file_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_target(true)
        .with_ansi(false)
        .with_span_events(fmt::format::FmtSpan::NEW | fmt::format::FmtSpan::CLOSE)
        .with_file(true)
        .with_line_number(true)
        .with_filter(filter);

    let subscriber = tracing_subscriber::registry()
        .with(stdout_layer)
        .with(file_layer);
    tracing::subscriber::set_global_default(subscriber)?;

    tracing::info!(target: "ninja::cli", "logger initialized" );
    Ok(())
}
