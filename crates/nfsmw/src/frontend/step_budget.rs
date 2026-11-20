//! The menu step budget: how long one step of the menus may take, and the words of the warning when it takes longer.
//!
//! A step of the menus runs on the frame thread, so a step over the budget is a visible hitch. The warning names the
//! screen the step started on, the phase of the step that took the time and, for a command a screen asked for, which one
//! (`docs/architecture.md`, "Menu step budget"). Nothing here allocates unless a step is over the budget.

use std::fmt::{self, Write};
use std::time::{Duration, Instant};

use super::logic::Command;

/// How long a menu step, or one command a screen asks for, may take before it is logged. 50 ms is three frames at 60 Hz:
/// a longer step is a hitch the player sees. Measured on the installed screens (options pages, My Cars, the career
/// screens, the pause menu) a step takes well under 2 ms, so the budget has a wide margin on a healthy run; what it
/// catches is a step that opens a scene or reads a file on the frame thread.
pub const MENU_STEP_BUDGET: Duration = Duration::from_millis(50);

/// A phase that took less than this is left out of the warning's list, so the list names only what matters.
const WORTH_NAMING: Duration = Duration::from_millis(1);

/// The most phases one step has (`Phases::record` ignores any beyond them).
const MAX_PHASES: usize = 8;

/// How many bytes of a command's name are kept: the longest variant name is shorter.
const HEAD_CAPACITY: usize = 32;

/// The phases of one menu step, each with the time it took, in the order they ran.
pub struct Phases {
    mark: Instant,
    spans: [(&'static str, Duration); MAX_PHASES],
    len: usize,
}

impl Phases {
    /// Starts the clock of a step.
    pub fn start() -> Self {
        Self { mark: Instant::now(), spans: [("", Duration::ZERO); MAX_PHASES], len: 0 }
    }

    /// Ends the phase `name`: it took what has passed since the step began or the last phase ended.
    pub fn end(&mut self, name: &'static str) {
        let now = Instant::now();
        let took = now.duration_since(self.mark);
        self.mark = now;
        self.record(name, took);
    }

    /// Adds a phase that took `took`.
    pub fn record(&mut self, name: &'static str, took: Duration) {
        if self.len == MAX_PHASES {
            return;
        }
        self.spans[self.len] = (name, took);
        self.len += 1;
    }

    /// The time of all the phases.
    pub fn total(&self) -> Duration {
        self.spans[..self.len].iter().map(|(_, took)| *took).sum()
    }

    /// The line to log for a step that started on `screen`, or `None` while the step is inside the budget.
    pub fn warning(&self, screen: Option<&str>) -> Option<String> {
        let total = self.total();
        if total < MENU_STEP_BUDGET {
            return None;
        }
        let mut line = String::new();
        let _ = write!(line, "a menu step took {} ms (on {})", total.as_millis(), screen.unwrap_or("no screen"));
        let mut parts = self.spans[..self.len].iter().filter(|(_, took)| *took >= WORTH_NAMING).peekable();
        if parts.peek().is_some() {
            line.push_str(": ");
        }
        for (index, (name, took)) in parts.enumerate() {
            let separator = match index {
                0 => "",
                _ => ", ",
            };
            let _ = write!(line, "{separator}{name} {} ms", took.as_millis());
        }
        Some(line)
    }
}

/// The first words of a command's name, in a buffer on the stack: the command is formatted only as far as the buffer
/// reaches, so a command with a big payload costs no more than a small one.
pub struct Head {
    text: [u8; HEAD_CAPACITY],
    len: usize,
}

impl Head {
    /// The name of `command`: its variant, cut where its payload begins.
    pub fn of(command: &Command) -> Self {
        let mut head = Self { text: [0; HEAD_CAPACITY], len: 0 };
        // The sink refuses what does not fit, which ends the formatting early; a full buffer is not an error here.
        let _ = write!(head, "{command:?}");
        head
    }

    /// The variant's name, without the payload.
    pub fn as_str(&self) -> &str {
        let text = std::str::from_utf8(&self.text[..self.len]).unwrap_or_default();
        let end = text.find(['(', ' ', '{']).unwrap_or(text.len());
        &text[..end]
    }
}

impl Write for Head {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let room = HEAD_CAPACITY - self.len;
        // Only whole characters are kept, so the buffer is always valid text.
        let mut take = text.len().min(room);
        while !text.is_char_boundary(take) {
            take -= 1;
        }
        self.text[self.len..self.len + take].copy_from_slice(&text.as_bytes()[..take]);
        self.len += take;
        match take == text.len() {
            true => Ok(()),
            false => Err(fmt::Error),
        }
    }
}

/// The line to log for a command that took `took`, or `None` while it is inside the budget.
pub fn command_warning(command: &Head, took: Duration) -> Option<String> {
    if took < MENU_STEP_BUDGET {
        return None;
    }
    Some(format!("a menu command took {} ms: {}", took.as_millis(), command.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(count: u64) -> Duration {
        Duration::from_millis(count)
    }

    #[test]
    fn a_step_inside_the_budget_is_not_logged() {
        let mut phases = Phases::start();
        phases.record("screens", ms(20));
        phases.record("commands", ms(29));
        assert_eq!(phases.warning(Some("MainMenu.fng")), None);
    }

    #[test]
    fn a_step_at_the_budget_is_logged_with_its_screen_and_the_phases_that_matter() {
        let mut phases = Phases::start();
        phases.record("screens", ms(10));
        phases.record("commands", ms(38));
        phases.record("backdrop", Duration::from_micros(300));
        phases.record("startup", ms(2));
        let line = phases.warning(Some("Pause_Main.fng")).expect("over the budget");
        assert_eq!(line, "a menu step took 50 ms (on Pause_Main.fng): screens 10 ms, commands 38 ms, startup 2 ms");
    }

    #[test]
    fn a_step_on_no_screen_says_so() {
        let mut phases = Phases::start();
        phases.record("commands", ms(120));
        assert_eq!(phases.warning(None).as_deref(), Some("a menu step took 120 ms (on no screen): commands 120 ms"));
    }

    #[test]
    fn the_total_is_the_sum_of_the_phases() {
        let mut phases = Phases::start();
        phases.record("screens", ms(3));
        phases.record("commands", ms(40));
        phases.record("garage", ms(7));
        assert_eq!(phases.total(), ms(50));
    }

    #[test]
    fn phases_past_the_limit_are_ignored() {
        let mut phases = Phases::start();
        (0..MAX_PHASES + 3).for_each(|_| phases.record("phase", ms(1)));
        assert_eq!(phases.total(), ms(MAX_PHASES as u64));
    }

    #[test]
    fn ending_a_phase_takes_the_time_since_the_last_one() {
        let mut phases = Phases::start();
        std::thread::sleep(ms(5));
        phases.end("screens");
        phases.end("commands");
        let (first, second) = (phases.spans[0], phases.spans[1]);
        assert_eq!((first.0, second.0), ("screens", "commands"));
        assert!(first.1 >= ms(5), "the first phase has the sleep: {:?}", first.1);
        assert!(second.1 < first.1, "the second phase starts where the first ended");
    }

    #[test]
    fn a_command_is_named_by_its_variant_whatever_its_payload() {
        assert_eq!(Head::of(&Command::OpenGarage).as_str(), "OpenGarage");
        let long = "a-very-long-alias-name-that-overflows-the-buffer".to_owned();
        assert_eq!(Head::of(&Command::CareerLoad(long)).as_str(), "CareerLoad");
        assert_eq!(Head::of(&Command::QuitTo(super::super::logic::QuitTo::Main)).as_str(), "QuitTo");
    }

    #[test]
    fn a_command_over_the_budget_is_logged_by_name() {
        let head = Head::of(&Command::OpenGarage);
        assert_eq!(command_warning(&head, ms(49)), None);
        assert_eq!(command_warning(&head, ms(80)).as_deref(), Some("a menu command took 80 ms: OpenGarage"));
    }
}
