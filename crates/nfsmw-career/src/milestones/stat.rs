//! The 30 tracked statistics, the best-value rule and the threshold test (`docs/specs/career-milestones.md` sections
//! 2.2, 3.3 and 3.5; the table of the statistics is `docs/specs/career-milestones-data.md` table 1).
//!
//! The type of each statistic (bigger or smaller is better) and the per-pursuit flag come from the data file
//! **[verified]**. Which statistics have a producer in the exe (section 3.4) is **[exe]**.

/// The number of tracked statistics (the rows of table 1).
pub const STAT_COUNT: usize = 30;

/// One of the 30 statistics of the `milestonetypes` collections. The discriminant is the row index in table 1, which is
/// also the index into the statistics array of [`super::MilestoneSet`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum StatType {
    /// Seconds of pursuit; bigger is better; per pursuit.
    PursuitLength = 0,
    /// Seconds to escape; smaller is better; per pursuit.
    PursuitEvasionTime = 1,
    /// Rep points of the pursuit; per pursuit.
    BountyInPursuit = 2,
    /// Cops wrecked in this pursuit; per pursuit.
    CopsDestroyedInPursuit = 3,
    /// Cops wrecked, running count; not per pursuit.
    CopsDestroyed = 4,
    /// Property damage value of the pursuit; per pursuit.
    CostToStateInPursuit = 5,
    /// Cost to state, running count; not per pursuit.
    CostToState = 6,
    /// Roadblocks dodged; per pursuit.
    RoadblocksDodged = 7,
    /// Cops tagged ("trade paint"); per pursuit.
    CopsDamaged = 8,
    /// Distinct infraction kinds in the pursuit; per pursuit.
    TotalInfractions = 9,
    /// Spike strips dodged; per pursuit.
    TireSpikesDodged = 10,
    /// Escapes in a row (the cop manager's counter); not per pursuit.
    PursuitsInARow = 11,
    /// Speed trap crossing speed in km/h; not per pursuit.
    SpeedtrapSpeed = 12,
    /// Ground cops in the pursuit; not per pursuit.
    CopsInPursuit = 13,
    /// Helicopters in the pursuit; not per pursuit.
    HelisInPursuit = 14,
    /// Cops joined in the pursuit so far; not per pursuit.
    TotalCopsInPursuit = 15,
    /// Collision claims; not per pursuit.
    InsuranceClaims = 16,
    /// Heat value; not per pursuit.
    HeatMeter = 17,
    /// No producer found in the exe.
    MarginOfVictory = 18,
    /// No producer found in the exe.
    HitAndRunTicket = 19,
    /// No producer found in the exe.
    DamagePropertyTicket = 20,
    /// No producer found in the exe.
    Call911 = 21,
    /// No producer found in the exe.
    OutrunPursuitsWon = 22,
    /// No producer found in the exe.
    FleeingTicket = 23,
    /// No producer found in the exe.
    OutrunRacesWon = 24,
    /// No producer found in the exe.
    DrivingSidewalkTicket = 25,
    /// No producer found in the exe.
    RacingTicket = 26,
    /// No producer found in the exe.
    SpeedingTicket = 27,
    /// No producer found in the exe.
    RecklessTicket = 28,
    /// No producer found in the exe.
    OpponentsDamaged = 29,
}

impl StatType {
    /// Every statistic, in table order.
    pub const ALL: [StatType; STAT_COUNT] = [
        StatType::PursuitLength,
        StatType::PursuitEvasionTime,
        StatType::BountyInPursuit,
        StatType::CopsDestroyedInPursuit,
        StatType::CopsDestroyed,
        StatType::CostToStateInPursuit,
        StatType::CostToState,
        StatType::RoadblocksDodged,
        StatType::CopsDamaged,
        StatType::TotalInfractions,
        StatType::TireSpikesDodged,
        StatType::PursuitsInARow,
        StatType::SpeedtrapSpeed,
        StatType::CopsInPursuit,
        StatType::HelisInPursuit,
        StatType::TotalCopsInPursuit,
        StatType::InsuranceClaims,
        StatType::HeatMeter,
        StatType::MarginOfVictory,
        StatType::HitAndRunTicket,
        StatType::DamagePropertyTicket,
        StatType::Call911,
        StatType::OutrunPursuitsWon,
        StatType::FleeingTicket,
        StatType::OutrunRacesWon,
        StatType::DrivingSidewalkTicket,
        StatType::RacingTicket,
        StatType::SpeedingTicket,
        StatType::RecklessTicket,
        StatType::OpponentsDamaged,
    ];

    /// The index into a statistics array (the table row).
    pub const fn index(self) -> usize {
        self as usize
    }

    /// The collection name in `GLOBAL/attributes.bin`.
    pub const fn name(self) -> &'static str {
        match self {
            StatType::PursuitLength => "pursuit_length",
            StatType::PursuitEvasionTime => "pursuit_evasion_time",
            StatType::BountyInPursuit => "bounty_in_pursuit",
            StatType::CopsDestroyedInPursuit => "cops_destroyed_in_pursuit",
            StatType::CopsDestroyed => "cops_destroyed",
            StatType::CostToStateInPursuit => "cost_to_state_in_pursuit",
            StatType::CostToState => "cost_to_state",
            StatType::RoadblocksDodged => "roadblocks_dodged",
            StatType::CopsDamaged => "cops_damaged",
            StatType::TotalInfractions => "total_infractions",
            StatType::TireSpikesDodged => "tire_spikes_dodged",
            StatType::PursuitsInARow => "pursuits_in_a_row",
            StatType::SpeedtrapSpeed => "speedtrap_speed",
            StatType::CopsInPursuit => "cops_in_pursuit",
            StatType::HelisInPursuit => "helis_in_pursuit",
            StatType::TotalCopsInPursuit => "total_cops_in_pursuit",
            StatType::InsuranceClaims => "insurance_claims",
            StatType::HeatMeter => "heat_meter",
            StatType::MarginOfVictory => "margin_of_victory",
            StatType::HitAndRunTicket => "hit_and_run_ticket",
            StatType::DamagePropertyTicket => "damage_property_ticket",
            StatType::Call911 => "911_calls",
            StatType::OutrunPursuitsWon => "outrun_pursuits_won",
            StatType::FleeingTicket => "fleeing_ticket",
            StatType::OutrunRacesWon => "outrun_races_won",
            StatType::DrivingSidewalkTicket => "driving_sidewalk_ticket",
            StatType::RacingTicket => "racing_ticket",
            StatType::SpeedingTicket => "speeding_ticket",
            StatType::RecklessTicket => "reckless_ticket",
            StatType::OpponentsDamaged => "opponents_damaged",
        }
    }

    /// True when bigger is better. Only `pursuit_evasion_time` (`MilestoneType` 2) is smaller-is-better.
    pub const fn bigger_is_better(self) -> bool {
        !matches!(self, StatType::PursuitEvasionTime)
    }

    /// The per-pursuit flag: both values are cleared at the start of each player pursuit (section 3.4).
    pub const fn per_pursuit(self) -> bool {
        matches!(
            self,
            StatType::PursuitLength
                | StatType::PursuitEvasionTime
                | StatType::BountyInPursuit
                | StatType::CopsDestroyedInPursuit
                | StatType::CostToStateInPursuit
                | StatType::RoadblocksDodged
                | StatType::CopsDamaged
                | StatType::TotalInfractions
                | StatType::TireSpikesDodged
        )
    }

    /// True for the 18 statistics that the exe updates (section 3.4). The other 12 are unused data.
    pub const fn has_producer(self) -> bool {
        !matches!(
            self,
            StatType::MarginOfVictory
                | StatType::HitAndRunTicket
                | StatType::DamagePropertyTicket
                | StatType::Call911
                | StatType::OutrunPursuitsWon
                | StatType::FleeingTicket
                | StatType::OutrunRacesWon
                | StatType::DrivingSidewalkTicket
                | StatType::RacingTicket
                | StatType::SpeedingTicket
                | StatType::RecklessTicket
                | StatType::OpponentsDamaged
        )
    }
}

/// The current and best value of one statistic. `None` is the `-1.0` the save uses for "not set".
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct StatValue {
    /// The last value written, better or not.
    pub current: Option<f32>,
    /// The best value so far in the statistic's direction.
    pub best: Option<f32>,
}

impl StatValue {
    /// Sets the current value and, when the value is better than the best (or no best is set yet), the best too.
    /// Returns true when the best changed. Only then the spec sends the progress message and tests the milestones
    /// (section 3.3, steps 2 to 5).
    pub fn record(&mut self, bigger_is_better: bool, value: f32) -> bool {
        self.current = Some(value);
        let improved = match self.best {
            None => true,
            Some(best) => is_better(bigger_is_better, value, best),
        };
        if improved {
            self.best = Some(value);
        }
        improved
    }

    /// Sets both values back to "not set".
    pub fn clear(&mut self) {
        *self = StatValue::default();
    }
}

/// True when `value` is strictly better than `than` in the statistic's direction. An equal value is not better.
pub fn is_better(bigger_is_better: bool, value: f32, than: f32) -> bool {
    match bigger_is_better {
        true => value > than,
        false => value < than,
    }
}

/// The threshold test of section 3.5: bigger-is-better passes at `value >= threshold`, smaller-is-better at
/// `value <= threshold` (the spec's difference `threshold - value`, with the sign flipped for smaller).
pub fn threshold_passed(bigger_is_better: bool, value: f32, threshold: f32) -> bool {
    match bigger_is_better {
        true => value >= threshold,
        false => value <= threshold,
    }
}
