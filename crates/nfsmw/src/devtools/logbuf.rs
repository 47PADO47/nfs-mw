//! A copy of the log, for the console to show. The same lines also go to the log file (see [`logfile`]).
//!
//! [`install`] replaces the plain `env_logger` with one that also keeps the last few hundred lines.
//! Console command output goes through [`output`], into the same buffer.

use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};

use log::{Level, Log, Metadata, Record};

use super::logfile;

/// Lines kept.
const CAPACITY: usize = 1000;

/// What a line is, for colouring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Log(Level),
    /// The result of a console command.
    Output,
    /// A command the user typed.
    Input,
    /// A command that failed.
    Error,
}

#[derive(Debug, Clone)]
pub struct Line {
    pub kind: Kind,
    pub text: String,
}

#[derive(Default)]
struct Buffer {
    lines: VecDeque<Line>,
}

static BUFFER: OnceLock<Mutex<Buffer>> = OnceLock::new();

/// Tests that read the shared buffer hold this, so one test's output is not another's.
#[cfg(test)]
pub(crate) static TEST_LOCK: Mutex<()> = Mutex::new(());

fn buffer() -> std::sync::MutexGuard<'static, Buffer> {
    BUFFER.get_or_init(Mutex::default).lock().unwrap_or_else(|e| e.into_inner())
}

fn push(kind: Kind, text: String) {
    let mut b = buffer();
    if b.lines.len() == CAPACITY {
        b.lines.pop_front();
    }
    b.lines.push_back(Line { kind, text });
}

/// Add a line of console output (one entry per line of `text`).
pub fn output(text: &str) {
    for line in text.lines() {
        push(Kind::Output, line.to_owned());
    }
}

/// Add an error from a console command.
pub fn error(text: &str) {
    push(Kind::Error, text.to_owned());
}

/// Add the command the user typed.
pub fn input(text: &str) {
    push(Kind::Input, text.to_owned());
}

/// Every line kept, oldest first.
pub fn lines() -> Vec<Line> {
    buffer().lines.iter().cloned().collect()
}

pub fn clear() {
    buffer().lines.clear();
}

/// `env_logger` on stderr, the same lines in the log file, plus the buffer.
struct Tee {
    stderr: env_logger::Logger,
    file: Option<env_logger::Logger>,
}

impl Log for Tee {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        self.stderr.enabled(metadata) || self.file.as_ref().is_some_and(|file| file.enabled(metadata))
    }

    fn log(&self, record: &Record<'_>) {
        if self.enabled(record.metadata()) {
            push(Kind::Log(record.level()), format!("{}", record.args()));
        }
        self.stderr.log(record);
        if let Some(file) = &self.file {
            file.log(record);
        }
    }

    fn flush(&self) {
        self.stderr.flush();
        if let Some(file) = &self.file {
            file.flush();
        }
    }
}

/// `gilrs::gamepad` warns once per device with no SDL mapping (virtual pads report an all-zero UUID); the pad still
/// works through its raw layout, so only its errors are shown. `discord_presence` logs an error each time the Discord
/// client is not running; the game's own `discord` lines say what matters.
const DEFAULT_FILTER: &str = "info,wgpu_core=warn,wgpu_hal=warn,naga=warn,gilrs::gamepad=error,discord_presence=off";

/// An `env_logger` builder that honours `RUST_LOG`, so the file and stderr keep the same filter.
fn env_builder() -> env_logger::Builder {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(DEFAULT_FILTER))
}

/// Set up logging: `env_logger`'s output (honouring `RUST_LOG`) on stderr, the same lines in the log file, plus the
/// console's copy. A panic is logged too, so it is in the file as well as on stderr.
pub fn install() {
    let stderr = env_builder().build();
    let opened = logfile::open();
    let log_path = opened.as_ref().map(|(path, _)| path.clone());
    let file = opened.map(|(_, file)| {
        env_builder()
            .write_style(env_logger::WriteStyle::Never)
            .target(env_logger::Target::Pipe(Box::new(file)))
            .build()
    });
    log::set_max_level(stderr.filter());
    // Only fails if a logger is already set, which we do not want to override.
    let _ = log::set_boxed_logger(Box::new(Tee { stderr, file }));
    if let Some(path) = log_path {
        log::info!("log file: {}", path.display());
    }
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log::error!("{info}");
        previous(info);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_splits_lines_and_the_buffer_is_bounded() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        clear();
        output("one\ntwo");
        let kept = lines();
        assert_eq!(kept.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(), ["one", "two"]);
        for i in 0..CAPACITY + 10 {
            output(&format!("line {i}"));
        }
        assert_eq!(lines().len(), CAPACITY);
        clear();
    }
}
