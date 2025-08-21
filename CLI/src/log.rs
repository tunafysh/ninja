use log::LevelFilter;
use file_rotate::{FileRotate, ContentLimit, suffix::AppendCount};
use std::io::Write;
use std::sync::Mutex;
use owo_colors::OwoColorize;

// Custom writer that wraps FileRotate
pub struct RotatingWriter {
    file_rotate: Mutex<FileRotate<AppendCount>>,
}

impl RotatingWriter {
    pub fn new(file_rotate: FileRotate<AppendCount>) -> Self {
        Self {
            file_rotate: Mutex::new(file_rotate),
        }
    }
}

impl Write for RotatingWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut file_rotate = self.file_rotate.lock().unwrap();
        file_rotate.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut file_rotate = self.file_rotate.lock().unwrap();
        file_rotate.flush()
    }
}

pub fn setup_logger(level: LevelFilter) -> Result<(), fern::InitError> {
    let log_path = match std::env::consts::OS {
        "linux" => format!("{}{}",std::env::var("HOME").expect("Failed to get environment variable"), "/.local/share/com.tunafysh.ninja/logs/shurikenctl.log"),
        "macos" => format!("{}{}",std::env::var("HOME").expect("Failed to get environment variable"), "/Library/Application Support/com.tunafysh.ninja/logs/shurikenctl.log"),
        "windows" => format!("{}{}",std::env::var("LOCALAPPDATA").expect("Failed to get environment variable"), "\\com.tunafysh.ninja\\logs\\shurikenctl.log"),
        _ => "logs/shurikenctl.log".to_string(),
    };

    // Ensure the directory exists
    if let Some(parent) = std::path::Path::new(&log_path).parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            fern::InitError::Io(e)
        })?;
    }

    // Configure log rotation
    let log_rotate = FileRotate::new(
        log_path,
        AppendCount::new(5),
        ContentLimit::Bytes(10_000_000),
        file_rotate::compression::Compression::None,
        None,
    );

    let rotating_writer = RotatingWriter::new(log_rotate);

    let colors = fern::colors::ColoredLevelConfig::new()
        .info(fern::colors::Color::Blue)
        .debug(fern::colors::Color::Cyan)
        .warn(fern::colors::Color::Yellow)
        .error(fern::colors::Color::Red);

    fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "[{}] [{}] {:15}: {}",
                chrono::Local::now().format("%d/%m/%Y %H:%M:%S"),
                colors.color(record.level()),
                record.target().magenta(),
                message
            ))
        })
        .level(level)
        .filter(|metadata| {
            // Block all logs from RustPython
            !metadata.target().starts_with("rustpython")
        })
        .chain(std::io::stdout())
        .chain(Box::new(rotating_writer) as Box<dyn Write + Send>)
        .apply()?;

    Ok(())
}
