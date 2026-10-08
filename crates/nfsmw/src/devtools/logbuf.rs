//! A copy of the log, for the console to show.
//!
//! [`install`] replaces the plain `env_logger` with one that also keeps the last few hundred lines.
//! Console command output goes through [`output`], into the same buffer.

use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};

use log::{Level, Log, Metadata, Record};

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

/// `env_logger` plus the buffer.
struct Tee(env_logger::Logger);

impl Log for Tee {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        self.0.enabled(metadata)
    }

    fn log(&self, record: &Record<'_>) {
        if self.enabled(record.metadata()) {
            push(Kind::Log(record.level()), format!("{}", record.args()));
        }
        self.0.log(record);
    }

    fn flush(&self) {
        self.0.flush();
    }
}

/// Set up logging: `env_logger`'s output (honouring `RUST_LOG`) plus the console's copy.
pub fn install() {
    let inner = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info,wgpu_core=warn,wgpu_hal=warn,naga=warn"),
    )
    .build();
    log::set_max_level(inner.filter());
    // Only fails if a logger is already set, which we do not want to override.
    let _ = log::set_boxed_logger(Box::new(Tee(inner)));
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
