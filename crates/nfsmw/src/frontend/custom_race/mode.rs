//! The six kinds of custom race the mode select lists, with their pictures and the labels each screen titles itself with
//! (`docs/specs/custom-race.md` section 1).

use nfsmw_data::gameplay::RaceType;

/// A kind of custom race, in the order the mode select lists them **[verified]** (the reference screenshots and the
/// order of `UIQRModeSelect::Setup`, with the tollbooth the original's PC build adds).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mode {
    Circuit,
    Sprint,
    Drag,
    Tollbooth,
    Knockout,
    SpeedTrap,
}

impl Mode {
    pub const ALL: [Mode; 6] =
        [Mode::Circuit, Mode::Sprint, Mode::Drag, Mode::Tollbooth, Mode::Knockout, Mode::SpeedTrap];

    /// The position in [`Mode::ALL`], which indexes the per-mode settings.
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|mode| *mode == self).unwrap_or(0)
    }

    /// The mode of a data race type; `None` for the types a custom race does not run (the pursuit challenges).
    #[cfg(test)]
    pub fn of(kind: RaceType) -> Option<Mode> {
        Some(match kind {
            RaceType::Circuit => Mode::Circuit,
            RaceType::Sprint => Mode::Sprint,
            RaceType::Drag => Mode::Drag,
            RaceType::Tollbooth => Mode::Tollbooth,
            RaceType::Knockout => Mode::Knockout,
            RaceType::SpeedTrap => Mode::SpeedTrap,
            _ => return None,
        })
    }

    pub fn race_type(self) -> RaceType {
        match self {
            Mode::Circuit => RaceType::Circuit,
            Mode::Sprint => RaceType::Sprint,
            Mode::Drag => RaceType::Drag,
            Mode::Tollbooth => RaceType::Tollbooth,
            Mode::Knockout => RaceType::Knockout,
            Mode::SpeedTrap => RaceType::SpeedTrap,
        }
    }

    /// The icon's texture (`MODE_ICON_*`) **[exe]**.
    pub fn icon(self) -> u32 {
        match self {
            Mode::Circuit => 0xE963_8D3E,
            Mode::Sprint => 0x2521_E5EB,
            Mode::Drag => 0xAAAB_31E9,
            Mode::Tollbooth => 0x1A09_1045,
            Mode::Knockout => 0x3A01_5595,
            Mode::SpeedTrap => 0x66C9_A7B6,
        }
    }

    /// The label of the mode's name: `Circuit`, `Sprint`, `Drag`, `Tollbooth`, `Lap Knockout`, `Speedtrap` **[exe]**.
    pub fn name(self) -> u32 {
        match self {
            Mode::Circuit => 0x034F_A2C1,
            Mode::Sprint => 0xB94F_D70E,
            Mode::Drag => 0x6F54_7E4C,
            Mode::Tollbooth => 0xA15E_4505,
            Mode::Knockout => 0x4930_F5FC,
            Mode::SpeedTrap => 0xEE1E_DC76,
        }
    }

    /// The label of the track list's title, `<mode> track select` **[exe]**.
    pub fn track_title(self) -> u32 {
        match self {
            Mode::Circuit => 0x3DE8_0A85,
            Mode::Sprint => 0xC2D8_5652,
            Mode::Drag => 0x136C_5C90,
            Mode::Tollbooth => 0xE3AF_ADC9,
            Mode::Knockout => 0xD6D6_5640,
            Mode::SpeedTrap => 0x3070_453A,
        }
    }

    /// The label of the options page's title, `<mode> options` **[exe]**.
    pub fn options_title(self) -> u32 {
        match self {
            Mode::Circuit => 0xA6ED_015D,
            Mode::Sprint => 0xB80B_FC8A,
            Mode::Drag => 0xEC86_E188,
            Mode::Tollbooth => 0x141E_DFE1,
            Mode::Knockout => 0x0CC9_59B8,
            Mode::SpeedTrap => 0xF274_5852,
        }
    }

    /// The tutorial movie under `MOVIES/` the mode select offers, if the install has one for the mode: drag, tollbooth and
    /// speedtrap **[verified]**.
    pub fn tutorial(self) -> Option<&'static str> {
        match self {
            Mode::Drag => Some("drag_tutorial"),
            Mode::Tollbooth => Some("tollbooth_tutorial"),
            Mode::SpeedTrap => Some("speedtrap_tutorial"),
            _ => None,
        }
    }

    /// The label of `Best Lap:` (circuit and knockout) or `Best Speed:` (speedtrap) the track list's record line starts with
    /// **[decomp]**; the other modes keep the package's `Best Time:`.
    pub fn record_label(self) -> Option<u32> {
        match self {
            Mode::Circuit | Mode::Knockout => Some(0xC5B5_A177),
            Mode::SpeedTrap => Some(0x0512_E823),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_modes_follow_the_menu_order_and_map_to_data_types() {
        assert_eq!(Mode::ALL.map(Mode::index), [0, 1, 2, 3, 4, 5]);
        for mode in Mode::ALL {
            assert_eq!(Mode::of(mode.race_type()), Some(mode));
        }
        assert_eq!(Mode::of(RaceType::Challenge), None);
    }

    #[test]
    fn only_drag_tollbooth_and_speedtrap_have_a_tutorial() {
        let with: Vec<Mode> = Mode::ALL.into_iter().filter(|m| m.tutorial().is_some()).collect();
        assert_eq!(with, [Mode::Drag, Mode::Tollbooth, Mode::SpeedTrap]);
    }
}
