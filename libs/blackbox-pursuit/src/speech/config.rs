//! What the machine is told about the game it runs in: the voice layout and the thresholds of the speech tuning. Both
//! come from the caller (the game's data); there are no defaults here.

use super::line::Speaker;
use super::params::Battalion;

/// A battalion and how many call sign numbers it has (numbers 1 to `size`, drawn without repeating from a shuffled
/// pool that is refilled when it runs out).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pool {
    pub battalion: Battalion,
    pub size: u8,
}

/// The voices and battalions of the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    pub dispatch: Speaker,
    pub heli: Speaker,
    /// The "primary" voices: the leader's, chosen first.
    pub primary: &'static [Speaker],
    /// The "secondary" voices: roadblock cars and the extra cops.
    pub secondary: &'static [Speaker],
    /// The voice only the cross (super pursuit) car takes.
    pub cross: Speaker,
    /// The helicopter's battalion and call sign number.
    pub heli_unit: (Battalion, u8),
    /// The call sign pools of the battalions that have one.
    pub pools: &'static [Pool],
    /// The battalions a cop on an unplaced road calls himself, one at random.
    pub random_battalions: &'static [Battalion],
    /// The first and last road speech id the machine can place a suspect on (a heading is only reported on these).
    pub placeable_roads: (u16, u16),
    /// The first and last road speech id the game can name a place for (a wider range than `placeable_roads`).
    pub locatable_roads: (u16, u16),
}

impl Layout {
    /// Every cop voice (primary, secondary, cross), in the order of the layout.
    pub fn cop_voices(&self) -> Vec<Speaker> {
        let mut all: Vec<Speaker> = self.primary.iter().chain(self.secondary).copied().collect();
        all.push(self.cross);
        all
    }

    pub fn is_primary(&self, speaker: Speaker) -> bool {
        self.primary.contains(&speaker) || speaker == self.heli || speaker == self.cross
    }

    pub fn is_secondary(&self, speaker: Speaker) -> bool {
        self.secondary.contains(&speaker)
    }

    pub fn pool_size(&self, battalion: Battalion) -> Option<u8> {
        self.pools.iter().find(|p| p.battalion == battalion).map(|p| p.size)
    }

    pub fn road_is_placeable(&self, road: u16) -> bool {
        road >= self.placeable_roads.0 && road <= self.placeable_roads.1
    }
}

/// The thresholds of the speech tuning (seconds, metres, miles per hour).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tune {
    pub pursuit_init_delay: f32,
    pub range_for_spotter: f32,
    pub speed_diff_for_blowby: f32,
    pub blowby_interval: f32,
    pub suspect_outrun_range: f32,
    pub time_considered_lost_no_los: f32,
    /// Seconds without contact before the pursuit counts as lost, then abandoned.
    pub inactivity: [f32; 2],
    pub no_los_commentary: f32,
    pub min_speed_stopped: f32,
    pub min_time_stopped: f32,
    pub high_intensity_heat: f32,
    pub duration_high_intensity: f32,
    pub min_bailout_duration: f32,
    pub min_health_commentary: f32,
    pub crash_slowdown: f32,
    pub smash_range: [f32; 2],
    pub min_intensity_traffic: f32,
    pub min_intensity_cop_smash: f32,
    pub min_havoc: f32,
    pub min_height_airborne: f32,
    pub hang_time: f32,
    pub flip_time: f32,
    pub fly_flip_speed: f32,
    pub contig_frames_180: u32,
    pub range_180: f32,
    pub time_180: f32,
    pub outcome_track_time: f32,
    pub outcome_fail_speed: f32,
    pub rb_outcome_timer: f32,
    pub rb_post_outcome_reset: f32,
    pub backup_remind_time: f32,
    pub racer_proximity: f32,
}
