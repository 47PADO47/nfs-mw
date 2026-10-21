//! The options page's rows on the widget menu: what each row of the current mode shows and does (`docs/specs/custom-race.md`
//! section 4). The values live in [`state`](super::state), not in the settings.

use super::mode::Mode;
use super::settings::{Row, Settings, rows as rows_of};
use super::state;
use crate::frontend::ids::{LABEL_OFF, LABEL_ON};
use crate::frontend::options::Data;

/// The rows of the current mode's page.
pub fn rows() -> Vec<Row> {
    rows_of(state::mode()).to_vec()
}

/// Whether a row can be changed: the lap knockout's laps follow its opponents and are greyed out **[exe]**.
pub fn enabled(row: Row) -> bool {
    !(row == Row::Laps && state::mode() == Mode::Knockout)
}

/// Moves a row of the current mode one value on.
pub fn step(row: Row, forward: bool) {
    state::with(|state| {
        let mode = state.mode;
        state.settings_mut().step(mode, row, forward);
    });
}

/// What a row shows now.
pub fn data(row: Row) -> Data {
    let settings = state::with(|state| *state.settings());
    value(row, &settings)
}

/// The value of a row: a number or a language label **[exe]**.
pub fn value(row: Row, settings: &Settings) -> Data {
    match row {
        Row::Laps => Data::Text(settings.laps.to_string()),
        Row::Opponents => Data::Text(settings.opponents.to_string()),
        Row::Difficulty => Data::Label(DIFFICULTY[usize::from(settings.difficulty.min(2))]),
        Row::Traffic => Data::Label(LEVEL[usize::from(settings.traffic.min(3))]),
        Row::Heat => Data::Label(LEVEL[usize::from(settings.heat.min(3))]),
        Row::CatchUp => Data::Label(if settings.catch_up { LABEL_ON } else { LABEL_OFF }),
    }
}

/// Easy, Medium, Hard.
const DIFFICULTY: [u32; 3] = [0x6197_3E01, 0x3747_F6D0, 0x6198_E2EE];
/// None, Minimum, Moderate, Maximum.
const LEVEL: [u32; 4] = [0x8CDC_3937, 0x73C6_15A3, 0xA2CC_A838, 0x61D1_C5A5];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rows_show_the_screenshots_defaults() {
        let s = Settings::default();
        assert_eq!(value(Row::Laps, &s), Data::Text("1".into()));
        assert_eq!(value(Row::Opponents, &s), Data::Text("3".into()));
        assert_eq!(value(Row::Difficulty, &s), Data::Label(0x3747_F6D0), "Medium");
        assert_eq!(value(Row::Traffic, &s), Data::Label(0x73C6_15A3), "Minimum");
        assert_eq!(value(Row::Heat, &s), Data::Label(0x73C6_15A3), "Minimum");
        assert_eq!(value(Row::CatchUp, &s), Data::Label(LABEL_ON));
    }
}
