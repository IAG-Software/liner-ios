//! Boot diagnostics.
//!
//! An iOS app that shows a black screen gives you nothing: no console, no
//! crash log unless it actually crashes, and `NSLog` only reachable from a Mac.
//! This module mirrors the `log` crate (and the native boot markers) into
//!
//!     <app>/Documents/liner-boot.log
//!
//! which is readable on-device via Files.app thanks to `UIFileSharingEnabled`.
//! The previous run is kept as `liner-boot.prev.log` so a crash on launch does
//! not destroy the evidence.

use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use once_cell::sync::OnceCell;
use tauri::{AppHandle, Manager};

static LOG_PATH: OnceCell<PathBuf> = OnceCell::new();
static WRITE_LOCK: Mutex<()> = Mutex::new(());

struct FileLogger;

impl log::Log for FileLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        let level = match record.level() {
            log::Level::Error => "ERROR",
            log::Level::Warn => "WARN ",
            log::Level::Info => "INFO ",
            log::Level::Debug => "DEBUG",
            log::Level::Trace => "TRACE",
        };
        append_line(&format!(
            "[{level}] {}: {}",
            record.target(),
            record.args()
        ));
    }

    fn flush(&self) {}
}

/// Append one line to the boot log (and to stderr, which Xcode would show).
pub fn append_line(message: &str) {
    eprintln!("{message}");

    let Some(path) = LOG_PATH.get() else {
        return;
    };
    let _guard = WRITE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(file, "{message}");
        let _ = file.flush();
    }
}

/// Resolve `<Documents>/liner-boot.log`, rotate the previous run, install the
/// logger and write the first marker.
pub fn init(app: &AppHandle) -> Option<PathBuf> {
    let documents = app
        .path()
        .document_dir()
        .or_else(|_| app.path().app_data_dir())
        .ok()?;

    let _ = std::fs::create_dir_all(&documents);
    let path = documents.join("liner-boot.log");
    let previous = documents.join("liner-boot.prev.log");

    if path.exists() {
        let _ = std::fs::rename(&path, &previous);
    }
    let _ = std::fs::write(&path, b"");
    let _ = LOG_PATH.set(path.clone());

    // Keep the previous run too, so a launch crash is still inspectable.
    if let Ok(prev) = std::fs::read_to_string(&previous) {
        if !prev.is_empty() {
            append_line("--- previous run tail ---");
            for line in prev.lines().rev().take(40).collect::<Vec<_>>().iter().rev() {
                append_line(line);
            }
            append_line("--- current run ---");
        }
    }

    static LOGGER: FileLogger = FileLogger;
    let _ = log::set_logger(&LOGGER);
    log::set_max_level(log::LevelFilter::Trace);

    append_line(&format!(
        "=== Liner Music {} boot ({}) ===",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS
    ));

    Some(path)
}

/// Path of the live log file, for the `boot_log_path` command.
pub fn path() -> Option<PathBuf> {
    LOG_PATH.get().cloned()
}
