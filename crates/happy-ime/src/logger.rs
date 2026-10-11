//! 轻量文件日志（%LOCALAPPDATA%\happy_shurufa\log.txt，超过 5MB 重建）。

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use log::{Level, LevelFilter, Metadata, Record};

struct FileLogger {
    file: Mutex<Option<File>>,
}

static LOGGER: OnceLock<FileLogger> = OnceLock::new();

pub fn setup() {
    let file = open_log_file();
    let logger = LOGGER.get_or_init(|| FileLogger {
        file: Mutex::new(file),
    });
    let _ = log::set_logger(logger);
    log::set_max_level(if cfg!(debug_assertions) {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    });
    log_panics();
}

/// 将 panic 记入日志（宿主进程崩溃排查的关键）。
pub fn log_panics() {
    std::panic::set_hook(Box::new(|info| {
        log::error!("panic: {info}");
    }));
}

fn log_file_path() -> Option<PathBuf> {
    let local = std::env::var("LOCALAPPDATA").ok()?;
    let dir = PathBuf::from(local).join("happy_shurufa");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("log.txt");
    // 超过 5MB 直接重建，避免无限增长
    if let Ok(meta) = std::fs::metadata(&path) {
        if meta.len() >= 5 * 1024 * 1024 {
            let _ = std::fs::remove_file(&path);
        }
    }
    Some(path)
}

fn open_log_file() -> Option<File> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file_path()?)
        .ok()
}

impl log::Log for FileLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let mut guard = match self.file.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        if guard.is_none() {
            *guard = open_log_file();
        }
        if let Some(file) = guard.as_mut() {
            let level = match record.level() {
                Level::Error => "ERR",
                Level::Warn => "WRN",
                Level::Info => "INF",
                Level::Debug => "DBG",
                Level::Trace => "TRC",
            };
            let _ = writeln!(file, "[{level}] [{}] {}", record.target(), record.args());
        }
    }

    fn flush(&self) {}
}
