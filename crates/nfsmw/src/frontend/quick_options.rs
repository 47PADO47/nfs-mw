//! Quick Race, step two: the four option rows of `Track_Options.fng` (laps, opponents, difficulty, traffic) and what
//! they choose. Each row steps through "the race's own value" and a few set ones; the choice goes to the race as
//! [`RaceOptions`] when the race starts. The choice is kept for the session, not written to the config file.
//! Spec: `docs/specs/race-mode.md` section 11.

use std::sync::Mutex;

use crate::scenes::world::RaceOptions;

/// A row of the track options page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuickRow {
    Laps,
    Opponents,
    Difficulty,
    Traffic,
}

impl QuickRow {
    pub const ALL: [QuickRow; 4] = [QuickRow::Laps, QuickRow::Opponents, QuickRow::Difficulty, QuickRow::Traffic];

    pub fn title(self) -> &'static str {
        match self {
            QuickRow::Laps => "Laps",
            QuickRow::Opponents => "Opponents",
            QuickRow::Difficulty => "Difficulty",
            QuickRow::Traffic => "Traffic",
        }
    }

    /// How many values the row cycles through; value 0 is "the race's own".
    fn count(self) -> usize {
        match self {
            QuickRow::Laps => 10,
            QuickRow::Opponents => 8,
            QuickRow::Difficulty => 4,
            QuickRow::Traffic => 5,
        }
    }
}

/// What each row is set to: 0 leaves the race's own value, a larger index sets one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Choice {
    laps: usize,
    opponents: usize,
    difficulty: usize,
    traffic: usize,
}

/// The difficulty values: the game's three classes (`DifficultyLevel` up to 33, 34 to 66 and above).
const DIFFICULTY: [(&str, i32); 3] = [("Easy", 33), ("Medium", 66), ("Hard", 100)];
/// The traffic values, as a share of the free-roam traffic budget (`ForceTrafficDensity`'s 0 to 100).
const TRAFFIC: [(&str, i32); 4] = [("None", 0), ("Light", 25), ("Medium", 50), ("Heavy", 100)];

static CHOICE: Mutex<Choice> = Mutex::new(Choice { laps: 0, opponents: 0, difficulty: 0, traffic: 0 });

fn lock() -> std::sync::MutexGuard<'static, Choice> {
    CHOICE.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The current choice.
pub fn choice() -> Choice {
    *lock()
}

impl Choice {
    fn index(&mut self, row: QuickRow) -> &mut usize {
        match row {
            QuickRow::Laps => &mut self.laps,
            QuickRow::Opponents => &mut self.opponents,
            QuickRow::Difficulty => &mut self.difficulty,
            QuickRow::Traffic => &mut self.traffic,
        }
    }

    /// The text a row shows.
    pub fn text(mut self, row: QuickRow) -> String {
        let at = *self.index(row);
        if at == 0 {
            return "Default".to_owned();
        }
        match row {
            QuickRow::Laps | QuickRow::Opponents => at.to_string(),
            QuickRow::Difficulty => DIFFICULTY[at - 1].0.to_owned(),
            QuickRow::Traffic => TRAFFIC[at - 1].0.to_owned(),
        }
    }

    /// Moves a row one value on, wrapping.
    pub fn step(&mut self, row: QuickRow, forward: bool) {
        let n = row.count();
        let at = self.index(row);
        *at = if forward { (*at + 1) % n } else { (*at + n - 1) % n };
    }

    /// What the choice asks of a race: the rows left on their default change nothing.
    pub fn race_options(self) -> RaceOptions {
        RaceOptions {
            laps: (self.laps > 0).then_some(self.laps as u32),
            opponents: (self.opponents > 0).then_some(self.opponents),
            difficulty: (self.difficulty > 0).then(|| DIFFICULTY[self.difficulty - 1].1),
            traffic: (self.traffic > 0).then(|| TRAFFIC[self.traffic - 1].1),
            ..Default::default()
        }
    }
}

/// Moves a row of the session's choice one value on.
pub fn step(row: QuickRow, forward: bool) {
    lock().step(row, forward);
}

/// The text of a row of the session's choice.
pub fn text(row: QuickRow) -> String {
    choice().text(row)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_choice_changes_nothing() {
        assert_eq!(Choice::default().race_options(), RaceOptions::default());
        assert_eq!(Choice::default().text(QuickRow::Laps), "Default");
    }

    #[test]
    fn each_row_sets_its_option_and_wraps() {
        let mut c = Choice::default();
        c.step(QuickRow::Laps, true);
        c.step(QuickRow::Laps, true);
        c.step(QuickRow::Opponents, false);
        c.step(QuickRow::Difficulty, true);
        c.step(QuickRow::Traffic, true);
        c.step(QuickRow::Traffic, true);
        let o = c.race_options();
        assert_eq!(o.laps, Some(2));
        assert_eq!(o.opponents, Some(7), "left from the default wraps to the last value");
        assert_eq!(o.difficulty, Some(33));
        assert_eq!(o.traffic, Some(25));
        assert_eq!(c.text(QuickRow::Difficulty), "Easy");
        for _ in 0..QuickRow::Laps.count() {
            c.step(QuickRow::Laps, true);
        }
        assert_eq!(c.race_options().laps, Some(2));
        c.step(QuickRow::Laps, false);
        c.step(QuickRow::Laps, false);
        assert_eq!(c.race_options().laps, None);
    }
}
