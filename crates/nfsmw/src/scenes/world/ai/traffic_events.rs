//! Traffic sound events and the rules that decide them: near misses, crash horns and the speeding trigger that
//! starts a pursuit. Spec: `docs/specs/traffic-sound-events.md`. The rules are pure functions of their inputs so
//! they can be tested without the simulation.

/// 65 mph in metres per second: the speed above which a cop on patrol reacts at heat 1 to 3.
pub const SPEEDING_MPS: f32 = 29.06;
/// Heat from which being seen is enough to start a pursuit.
const SEEN_IS_ENOUGH_HEAT: u32 = 4;

/// The thresholds the rules use. The defaults are the verified values where the spec has one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EventTuning {
    /// m/s above which a patrol cop reacts to the player's speed.
    pub speeding: f32,
    /// m/s the player must be doing for a pass to count as a near miss.
    pub near_miss_speed: f32,
    /// Lateral metres between the two bodies that count as a near miss.
    pub near_miss_gap: f32,
    /// Seconds before the same car can play another near miss or crash horn.
    pub cooldown: f32,
    /// Seconds a traffic hit still counts as recent.
    pub hit_memory: f32,
    /// Metres within which a patrol cop can see the player (heat 1 to 7 front sight, ai-pursuit §4.1).
    pub cop_sight: f32,
}

impl Default for EventTuning {
    fn default() -> Self {
        Self {
            speeding: SPEEDING_MPS,
            near_miss_speed: 20.0,
            near_miss_gap: 2.5,
            cooldown: 3.0,
            hit_memory: 1.0,
            cop_sight: 151.0,
        }
    }
}

/// A fact about a pass or a hit. The rules below decide what each one plays.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrafficEvent {
    /// The player passed a traffic car close by without touching it.
    NearMiss { speed: f32, gap: f32 },
    /// A car hit another car or the player; `horn_id` is its `pvehicle.HornType`.
    CrashHorn { horn_id: u32, impulse: f32, by_player: bool },
}

/// Whether a near miss plays: the player is fast enough, the gap is small and positive, the pass was not a hit,
/// and the car's cooldown has run out (`ready`).
pub fn near_miss(tuning: &EventTuning, player_speed: f32, gap: f32, hit: bool, ready: bool) -> bool {
    !hit && ready && player_speed >= tuning.near_miss_speed && gap > 0.0 && gap <= tuning.near_miss_gap
}

/// Whether a car's horn plays for a hit: the car is involved and its cooldown has run out (`ready`).
pub fn crash_horn(involved: bool, ready: bool) -> bool {
    involved && ready
}

/// What a cop on patrol knows when it decides whether to chase the player.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sighting {
    /// Heat of the current pursuit table.
    pub heat: u32,
    /// Player speed, m/s.
    pub player_speed: f32,
    /// The cop can see the player (`CanSeeTarget`).
    pub seen: bool,
    /// The player hit traffic within `hit_memory` seconds.
    pub recent_hit: bool,
    /// A 911 call about the player is active.
    pub has_911: bool,
}

/// Whether a patrol cop starts a pursuit (`WatchForPerps`, ai-pursuit §4.2).
pub fn starts_pursuit(tuning: &EventTuning, sight: &Sighting) -> bool {
    if !sight.seen {
        return false;
    }
    if sight.heat >= SEEN_IS_ENOUGH_HEAT {
        return true;
    }
    sight.player_speed > tuning.speeding || sight.recent_hit || sight.has_911
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sight(heat: u32, speed: f32, seen: bool) -> Sighting {
        Sighting { heat, player_speed: speed, seen, recent_hit: false, has_911: false }
    }

    #[test]
    fn near_miss_needs_speed_gap_no_hit_and_ready() {
        let t = EventTuning::default();
        assert!(near_miss(&t, 25.0, 1.0, false, true));
        assert!(!near_miss(&t, 19.9, 1.0, false, true), "slow pass");
        assert!(!near_miss(&t, 25.0, 0.0, false, true), "touching is not a near miss");
        assert!(!near_miss(&t, 25.0, 2.6, false, true), "too far");
        assert!(!near_miss(&t, 25.0, 1.0, true, true), "a hit plays the horn instead");
        assert!(!near_miss(&t, 25.0, 1.0, false, false), "cooldown");
    }

    #[test]
    fn near_miss_gap_edge_is_inclusive() {
        let t = EventTuning::default();
        assert!(near_miss(&t, 25.0, t.near_miss_gap, false, true));
    }

    #[test]
    fn crash_horn_needs_involvement_and_ready() {
        assert!(crash_horn(true, true));
        assert!(!crash_horn(false, true));
        assert!(!crash_horn(true, false));
    }

    #[test]
    fn patrol_cop_ignores_slow_unhit_player_at_low_heat() {
        let t = EventTuning::default();
        assert!(!starts_pursuit(&t, &sight(3, SPEEDING_MPS - 0.1, true)));
    }

    #[test]
    fn speeding_over_65_mph_starts_pursuit_at_low_heat() {
        let t = EventTuning::default();
        assert!(starts_pursuit(&t, &sight(3, SPEEDING_MPS + 0.1, true)));
        assert!(!starts_pursuit(&t, &sight(3, SPEEDING_MPS + 0.1, false)), "must be seen");
    }

    #[test]
    fn recent_hit_or_911_starts_pursuit_at_low_heat() {
        let t = EventTuning::default();
        let hit = Sighting { recent_hit: true, ..sight(1, 5.0, true) };
        let call = Sighting { has_911: true, ..sight(1, 5.0, true) };
        assert!(starts_pursuit(&t, &hit));
        assert!(starts_pursuit(&t, &call));
    }

    #[test]
    fn being_seen_is_enough_from_heat_4() {
        let t = EventTuning::default();
        assert!(starts_pursuit(&t, &sight(4, 5.0, true)));
        assert!(!starts_pursuit(&t, &sight(3, 5.0, true)));
    }
}
