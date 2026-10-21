//! The options of a custom race, one set per mode, and what they ask of the race (`docs/specs/custom-race.md` section 4).
//! The rows are the exe's: `UIQRTrackOptions::Setup*` builds them from the widgets `NumLaps`, `TrafficLevel`,
//! `NumOpponents`, `AISkill` and `CatchUp` **[exe]**, and the PC build adds Heat Level at the end **[verified]** (the
//! reference screenshot of the circuit options).

use super::mode::Mode;
use crate::scenes::world::RaceOptions;

/// A row of the options page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row {
    Laps,
    Traffic,
    Opponents,
    Difficulty,
    CatchUp,
    Heat,
}

impl Row {
    /// The language label of the row's title **[exe]**.
    pub fn title(self) -> u32 {
        match self {
            Row::Laps => 0x4849_4E83,
            Row::Traffic => 0xEB9D_FC09,
            Row::Opponents => 0x3384_A679,
            Row::Difficulty => 0x4D15_6786,
            Row::CatchUp => 0x8B8E_913A,
            Row::Heat => 0x251A_34F8,
        }
    }
}

/// The rows of a mode, in the order the page lists them **[exe]**: the circuit has laps first; the lap knockout lists its
/// laps greyed out after the opponents (they follow the opponents); the tollbooth has traffic only. No race of this
/// install can run reversed (`racestartReverse` is 0 everywhere), so the Track Direction row never shows.
pub fn rows(mode: Mode) -> &'static [Row] {
    use Row::*;
    match mode {
        Mode::Circuit => &[Laps, Traffic, Opponents, Difficulty, CatchUp, Heat],
        Mode::Sprint | Mode::SpeedTrap => &[Traffic, Opponents, Difficulty, CatchUp, Heat],
        Mode::Drag => &[Opponents, Difficulty, CatchUp, Heat],
        Mode::Knockout => &[Opponents, Laps, Traffic, Difficulty, CatchUp, Heat],
        Mode::Tollbooth => &[Traffic, Heat],
    }
}

/// The rows of a mode's page in split screen **[decomp]** (`UIQRTrackOptions::Setup*` with `IsSplitScreenMode`): the circuit
/// keeps its laps; every other mode has no row left, as Direction never shows in this install. Opponents, Difficulty and
/// Catch Up are fixed, Traffic is not offered, and Heat Level (this build's own row) is not either: there are no cops.
pub fn split_rows(mode: Mode) -> &'static [Row] {
    match mode {
        Mode::Circuit => &[Row::Laps],
        _ => &[],
    }
}

/// Most laps a circuit offers **[decomp]**.
pub const MAX_LAPS: u8 = 8;
/// Most opponents: three, the grid of four less the player **[decomp]**.
pub const MAX_OPPONENTS: u8 = 3;

/// What the options of one mode are set to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub laps: u8,
    pub opponents: u8,
    /// 0 Easy, 1 Medium, 2 Hard.
    pub difficulty: u8,
    /// 0 None, 1 Minimum, 2 Moderate, 3 Maximum.
    pub traffic: u8,
    pub catch_up: bool,
    /// 0 None, 1 Minimum, 2 Moderate, 3 Maximum.
    pub heat: u8,
}

impl Default for Settings {
    /// The defaults of `DefaultRaceSettings` and the reference screenshot: one lap, three opponents, medium difficulty,
    /// minimum traffic, catch up on, minimum heat.
    fn default() -> Self {
        Self { laps: 1, opponents: MAX_OPPONENTS, difficulty: 1, traffic: 1, catch_up: true, heat: 1 }
    }
}

/// The traffic density a level stands for, as `FillCustomRace` writes it **[exe]**.
const TRAFFIC_DENSITY: [i32; 4] = [0, 10, 30, 50];
/// The difficulty a level stands for (the quick race's 33, 66 and 100).
const DIFFICULTY: [i32; 3] = [33, 66, 100];

impl Settings {
    /// Settings with the lap knockout's rule applied: its laps are its opponents.
    pub fn defaults_for(mode: Mode) -> Self {
        let mut settings = Self::default();
        if mode == Mode::Knockout {
            settings.laps = settings.opponents;
        }
        settings
    }

    /// Moves a row one value on, wrapping; the lap knockout's laps follow its opponents instead.
    pub fn step(&mut self, mode: Mode, row: Row, forward: bool) {
        let cycle = |at: u8, first: u8, last: u8| match forward {
            true if at >= last => first,
            true => at + 1,
            false if at <= first => last,
            false => at - 1,
        };
        match row {
            Row::Laps if mode == Mode::Knockout => {}
            Row::Laps => self.laps = cycle(self.laps, 1, MAX_LAPS),
            Row::Opponents => self.opponents = cycle(self.opponents, 1, MAX_OPPONENTS),
            Row::Difficulty => self.difficulty = cycle(self.difficulty, 0, 2),
            Row::Traffic => self.traffic = cycle(self.traffic, 0, 3),
            Row::CatchUp => self.catch_up = !self.catch_up,
            Row::Heat => self.heat = cycle(self.heat, 0, 3),
        }
        if mode == Mode::Knockout {
            self.laps = self.opponents;
        }
    }

    /// What the settings ask of the race. The laps are the circuit's and the knockout's (every other race runs its own);
    /// the knockout runs as many laps as it has opponents, so one racer is out each lap **[decomp]**.
    pub fn race_options(&self, mode: Mode) -> RaceOptions {
        let laps = match mode {
            Mode::Circuit => Some(u32::from(self.laps)),
            Mode::Knockout => Some(u32::from(self.opponents)),
            _ => None,
        };
        let has = |row: Row| rows(mode).contains(&row);
        RaceOptions {
            laps,
            opponents: has(Row::Opponents).then_some(usize::from(self.opponents)),
            difficulty: has(Row::Difficulty).then(|| DIFFICULTY[usize::from(self.difficulty.min(2))]),
            traffic: Some(TRAFFIC_DENSITY[usize::from(self.traffic.min(3))]),
            catch_up: has(Row::CatchUp).then_some(self.catch_up),
            heat: Some(self.heat.min(3)),
            ..Default::default()
        }
    }
}

impl Settings {
    /// What the settings ask of a split-screen race: the second player is the one opponent, the police are off
    /// (`SetNumOpponents(1)`, `SetCopsEnabled(false)` **[decomp]**) and nothing slows the leader between two humans
    /// (catch-up off **[ours]**). The traffic stays the race's own: the page does not offer it. The circuit's laps are the
    /// only row left.
    pub fn split_options(&self, mode: Mode) -> RaceOptions {
        RaceOptions {
            laps: (mode == Mode::Circuit).then_some(u32::from(self.laps)),
            opponents: Some(1),
            catch_up: Some(false),
            heat: Some(0),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rows_follow_the_exes_setup_per_mode() {
        assert_eq!(rows(Mode::Circuit)[0], Row::Laps);
        assert_eq!(rows(Mode::Tollbooth), [Row::Traffic, Row::Heat]);
        assert!(!rows(Mode::Drag).contains(&Row::Traffic));
        assert!(!rows(Mode::Sprint).contains(&Row::Laps));
        assert_eq!(rows(Mode::Knockout)[1], Row::Laps);
    }

    #[test]
    fn a_row_wraps_both_ways() {
        let mut s = Settings::default();
        s.step(Mode::Circuit, Row::Laps, false);
        assert_eq!(s.laps, MAX_LAPS);
        s.step(Mode::Circuit, Row::Laps, true);
        assert_eq!(s.laps, 1);
        s.step(Mode::Circuit, Row::Opponents, true);
        assert_eq!(s.opponents, 1);
        for _ in 0..3 {
            s.step(Mode::Circuit, Row::Difficulty, true);
        }
        assert_eq!(s.difficulty, 1);
        s.step(Mode::Circuit, Row::CatchUp, true);
        assert!(!s.catch_up);
    }

    #[test]
    fn the_knockouts_laps_follow_its_opponents_and_cannot_be_stepped() {
        let mut s = Settings::defaults_for(Mode::Knockout);
        assert_eq!(s.laps, 3);
        s.step(Mode::Knockout, Row::Opponents, false);
        assert_eq!((s.opponents, s.laps), (2, 2));
        s.step(Mode::Knockout, Row::Laps, true);
        assert_eq!(s.laps, 2);
        assert_eq!(s.race_options(Mode::Knockout).laps, Some(2));
    }

    #[test]
    fn the_options_a_mode_has_no_row_for_are_left_to_the_race() {
        let options = Settings::default().race_options(Mode::Tollbooth);
        assert_eq!((options.opponents, options.difficulty, options.catch_up, options.laps), (None, None, None, None));
        assert_eq!(options.traffic, Some(10));
        let circuit = Settings::default().race_options(Mode::Circuit);
        assert_eq!((circuit.laps, circuit.opponents, circuit.difficulty), (Some(1), Some(3), Some(66)));
        assert_eq!(Settings::default().race_options(Mode::Drag).laps, None);
    }

    #[test]
    fn a_split_screen_race_has_one_opponent_no_cops_and_only_the_circuits_laps() {
        let settings = Settings { laps: 4, opponents: 3, ..Settings::default() };
        let circuit = settings.split_options(Mode::Circuit);
        assert_eq!(
            (circuit.laps, circuit.opponents, circuit.catch_up, circuit.heat),
            (Some(4), Some(1), Some(false), Some(0))
        );
        assert_eq!(
            (circuit.difficulty, circuit.traffic),
            (None, None),
            "the race keeps its own traffic and difficulty"
        );
        assert_eq!(settings.split_options(Mode::Sprint).laps, None);
        assert_eq!(split_rows(Mode::Circuit), [Row::Laps]);
        assert!(split_rows(Mode::Drag).is_empty() && split_rows(Mode::SpeedTrap).is_empty());
    }
}
