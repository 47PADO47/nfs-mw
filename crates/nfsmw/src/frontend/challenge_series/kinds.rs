//! The kinds of challenge the series lists: a race with no statistic (the tollbooth time trials and the burger-chain
//! event) and the eight pursuit statistics. Each kind has its type key, its picture and the tutorial the screen can show
//! (`docs/specs/challenge-series.md` section 3).

use blackbox_attrib::vlt_hash;

/// The picture of an event that has no statistic: the tollbooth (`GetMilestoneIconHash` is not asked for it).
pub const RACE_ICON: u32 = 0x6581_8EE8;

/// The statistic a pursuit challenge asks for. The type key is the attribute hash of the data's `MilestoneName`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Milestone {
    pub name: &'static str,
    /// `vlt_hash(name)`: what `GRaceParameters::GetChallengeType` answers.
    pub key: u32,
    /// The picture of the statistic (`cFrontendDatabase::GetMilestoneIconHash`).
    pub icon: u32,
    /// The goal is a time in seconds, shown in minutes (`IsMilestoneTimeFormat`).
    pub time: bool,
    /// The tutorial movie the screen offers for it, if any (see [`Kind::tutorial`]).
    pub tutorial: Option<&'static str>,
}

const fn milestone(name: &'static str, icon: u32, time: bool, tutorial: Option<&'static str>) -> Milestone {
    Milestone { name, key: vlt_hash(name), icon, time, tutorial }
}

/// The eight statistics of the install's challenges, by `MilestoneName`.
pub const MILESTONES: [Milestone; 8] = [
    milestone("pursuit_length", 0x950F_CEBC, true, Some("pursuit_tutorial")),
    milestone("pursuit_evasion_time", 0x831B_7EBE, true, Some("pursuit_tutorial")),
    milestone("cops_damaged", 0x88E8_DE9E, false, None),
    milestone("cost_to_state_in_pursuit", 0x8780_7869, false, None),
    milestone("bounty_in_pursuit", 0xB4E6_456B, false, Some("bounty_tutorial")),
    milestone("total_infractions", 0x6784_A80E, false, None),
    milestone("roadblocks_dodged", 0xC439_59D2, false, None),
    milestone("tire_spikes_dodged", 0xE621_B2EF, false, None),
];

/// What an entry of the series is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A race with a time limit and no statistic: a tollbooth trial, or the burger-chain race.
    Race,
    /// A pursuit that has to reach a statistic and get away.
    Pursuit(Milestone),
}

impl Kind {
    /// The kind of a race by its `MilestoneName` (none is a race). An unknown statistic is a race too: the original
    /// answers type 0 for anything it does not list.
    pub fn of(milestone: Option<&str>) -> Self {
        let found = milestone.and_then(|name| MILESTONES.iter().find(|m| m.name == name));
        found.map_or(Kind::Race, |m| Kind::Pursuit(*m))
    }

    /// The picture of the kind.
    pub fn icon(self) -> u32 {
        match self {
            Kind::Race => RACE_ICON,
            Kind::Pursuit(m) => m.icon,
        }
    }

    /// Whether the goal reads as a time in minutes.
    pub fn time_goal(self) -> bool {
        matches!(self, Kind::Pursuit(m) if m.time)
    }

    /// The tutorial movie of the kind. The executable's handler of the tutorial button opens one only for a race
    /// (`tollbooth_tutorial`); this build also offers the pursuit and bounty tutorials for the statistics they
    /// explain **[ours]**.
    pub fn tutorial(self) -> Option<&'static str> {
        match self {
            Kind::Race => Some("tollbooth_tutorial"),
            Kind::Pursuit(m) => m.tutorial,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_type_keys_are_the_ones_the_executable_switches_on() {
        let key = |name: &str| MILESTONES.iter().find(|m| m.name == name).map(|m| m.key);
        assert_eq!(key("pursuit_length"), Some(0x033F_A23A));
        assert_eq!(key("pursuit_evasion_time"), Some(0x5392_E4FD));
        assert_eq!(key("cops_damaged"), Some(0x850A_64BC));
        assert_eq!(key("bounty_in_pursuit"), Some(0x2377_E50D));
        assert_eq!(key("tire_spikes_dodged"), Some(0xCDF3_6FC2));
    }

    #[test]
    fn only_the_two_time_statistics_read_as_minutes() {
        let times: Vec<&str> = MILESTONES.iter().filter(|m| m.time).map(|m| m.name).collect();
        assert_eq!(times, ["pursuit_length", "pursuit_evasion_time"]);
    }

    #[test]
    fn a_race_without_a_statistic_is_the_tollbooth_kind() {
        assert_eq!(Kind::of(None), Kind::Race);
        assert_eq!(Kind::of(Some("something_new")), Kind::Race);
        assert_eq!(Kind::of(Some("cops_damaged")).icon(), 0x88E8_DE9E);
        assert_eq!(Kind::Race.tutorial(), Some("tollbooth_tutorial"));
        assert_eq!(Kind::of(Some("cops_damaged")).tutorial(), None);
    }
}
