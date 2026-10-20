//! Quick Race, step two: the four option rows of `Track_Options.fng` (laps, opponents, difficulty, traffic) and what
//! they choose. Each row steps through "the race's own value" and a few set ones; the choice goes to the race as
//! [`RaceOptions`] when the race starts. The choice is a setting, saved with the others (`quick_race` in the config file).
//! Spec: `docs/specs/race-mode.md` section 11.

use crate::scenes::world::RaceOptions;
use crate::settings::{Partial, Settings};

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

/// What each row is set to: 0 leaves the race's own value, a larger index sets one. It is kept in the settings
/// (`Settings::quick_race`, config key `quick_race`), in the order laps, opponents, difficulty, traffic.
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

impl Choice {
    /// The choice the settings hold.
    pub fn of(settings: &Settings) -> Self {
        let [laps, opponents, difficulty, traffic] = settings.quick_race.map(usize::from);
        Self { laps, opponents, difficulty, traffic }
    }

    fn index(&mut self, row: QuickRow) -> &mut usize {
        match row {
            QuickRow::Laps => &mut self.laps,
            QuickRow::Opponents => &mut self.opponents,
            QuickRow::Difficulty => &mut self.difficulty,
            QuickRow::Traffic => &mut self.traffic,
        }
    }

    /// The text a row shows. An index past the row's values reads as the default.
    pub fn text(mut self, row: QuickRow) -> String {
        let at = *self.index(row);
        if at == 0 || at >= row.count() {
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

    /// What the choice asks of a race: the rows left on their default (or past their values) change nothing.
    pub fn race_options(self) -> RaceOptions {
        let within = |at: usize, row: QuickRow| (at > 0 && at < row.count()).then_some(at);
        RaceOptions {
            laps: within(self.laps, QuickRow::Laps).map(|n| n as u32),
            opponents: within(self.opponents, QuickRow::Opponents),
            difficulty: within(self.difficulty, QuickRow::Difficulty).map(|n| DIFFICULTY[n - 1].1),
            traffic: within(self.traffic, QuickRow::Traffic).map(|n| TRAFFIC[n - 1].1),
            ..Default::default()
        }
    }

    /// The choice as the settings keep it.
    fn stored(self) -> [u8; 4] {
        [self.laps, self.opponents, self.difficulty, self.traffic].map(|n| n.min(usize::from(u8::MAX)) as u8)
    }
}

/// Moves a row of the settings' choice one value on and records the change for the config file.
pub fn step(settings: &mut Settings, changed: &mut Partial, row: QuickRow, forward: bool) {
    let mut choice = Choice::of(settings);
    choice.step(row, forward);
    settings.quick_race = choice.stored();
    changed.quick_race = Some(settings.quick_race);
}

/// The text of a row of the settings' choice.
pub fn text(settings: &Settings, row: QuickRow) -> String {
    Choice::of(settings).text(row)
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
    fn a_step_goes_to_the_settings_and_the_config_change() {
        let mut settings = Settings::from(Partial::default());
        let mut changed = Partial::default();
        assert_eq!(settings.quick_race, [0; 4]);
        step(&mut settings, &mut changed, QuickRow::Difficulty, true);
        step(&mut settings, &mut changed, QuickRow::Laps, true);
        assert_eq!(settings.quick_race, [1, 0, 1, 0]);
        assert_eq!(changed.quick_race, Some([1, 0, 1, 0]));
        assert_eq!(text(&settings, QuickRow::Difficulty), "Easy");
        assert_eq!(Choice::of(&settings).race_options().laps, Some(1));
    }

    #[test]
    fn a_stored_index_past_a_rows_values_reads_as_the_default() {
        let settings = Settings::from(Partial { quick_race: Some([9, 9, 9, 9]), ..Default::default() });
        let c = Choice::of(&settings);
        assert_eq!(c.race_options().laps, Some(9), "laps run to 9");
        assert_eq!(c.race_options().difficulty, None, "past the three classes");
        assert_eq!(c.text(QuickRow::Traffic), "Default");
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
