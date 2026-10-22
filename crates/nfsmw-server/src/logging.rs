//! A small logger for the `log` facade: every record goes to stderr and, when configured, to a file, as
//! `[seconds.millis LEVEL target] message` with the time since the logger started. No new dependency: the file is
//! written through a mutex and flushed per line, which is enough for a server's few lines per second.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::Mutex;
use std::time::Instant;

use log::{LevelFilter, Log, Metadata, Record};

/// The logger behind the facade.
struct ServerLogger {
    level: LevelFilter,
    start: Instant,
    file: Option<Mutex<File>>,
}

impl Log for ServerLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let elapsed = self.start.elapsed();
        let line = format!(
            "[{:>6}.{:03} {:<5} {}] {}\n",
            elapsed.as_secs(),
            elapsed.subsec_millis(),
            record.level(),
            record.target(),
            record.args()
        );
        let _ = io::stderr().write_all(line.as_bytes());
        let Some(file) = &self.file else {
            return;
        };
        if let Ok(mut file) = file.lock() {
            let _ = file.write_all(line.as_bytes());
        }
    }

    fn flush(&self) {
        let Some(file) = &self.file else {
            return;
        };
        if let Ok(mut file) = file.lock() {
            let _ = file.flush();
        }
    }
}

/// Installs the logger: stderr, plus `file` opened for appending when given. Fails when the file cannot be opened or
/// a logger is installed already.
pub fn init(level: LevelFilter, file: Option<&Path>) -> io::Result<()> {
    let file = match file {
        Some(path) => Some(Mutex::new(OpenOptions::new().create(true).append(true).open(path)?)),
        None => None,
    };
    let logger = Box::leak(Box::new(ServerLogger { level, start: Instant::now(), file }));
    log::set_logger(logger).map_err(|err| io::Error::other(err.to_string()))?;
    log::set_max_level(level);
    Ok(())
}
