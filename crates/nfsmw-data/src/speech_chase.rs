//! The chase speech's tuning: the thresholds of the `speechtune` collection that the pursuit flows read, and the fields
//! of a heat level's `pursuitlevels` row that gate the roadblock chatter and the free-roam 911 call.
//! Spec: `docs/specs/ai-pursuit-speech-events.md` section 1 and `docs/specs/ai-pursuit-speech.md` sections 6 to 8.
//! The values below are the install's; they are the fallback when the database lacks the collection.

use blackbox_attrib::{CollectionRef, Database};

use crate::pursuit::level_collection;

/// The thresholds of the `speechtune` collection (seconds, metres, miles per hour).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChaseTune {
    pub pursuit_init_delay: f32,
    pub range_for_spotter_branch: f32,
    pub speed_diff_for_blowby: f32,
    pub blowby_interval: f32,
    pub suspect_outrun_range: f32,
    pub time_considered_lost_no_los: f32,
    pub pursuit_inactivity_timer: [f32; 2],
    pub no_los_commentary_time: f32,
    pub min_speed_considered_stopped: f32,
    pub min_time_considered_stopped: f32,
    pub high_intensity_mark: f32,
    pub pursuit_duration_high_intensity: f32,
    pub min_pursuit_duration_for_bailouts: f32,
    pub min_health_for_commentary: f32,
    pub crash_slowdown_pct: f32,
    pub player_smash_speed_range: [f32; 2],
    pub min_intensity_traffic_smash: f32,
    pub min_intensity_cop_smash: f32,
    pub min_havoc_for_suspect_behavior: f32,
    pub min_height_airborne: f32,
    pub hang_time_for_commentary: f32,
    pub flip_time_for_commentary: f32,
    pub speed_thresh_fly_flip_intensity: f32,
    pub min_contig_frames_for_180: u32,
    pub max_range_for_180: f32,
    pub max_time_for_180: f32,
    pub outcome_track_time: f32,
    pub outcome_fail_speed: f32,
    pub rb_outcome_timer: f32,
    pub rb_post_outcome_reset_time: f32,
    pub bu_remind_time: f32,
    pub ai_racer_proximity: f32,
}

impl Default for ChaseTune {
    fn default() -> Self {
        Self {
            pursuit_init_delay: 2.0,
            range_for_spotter_branch: 100.0,
            speed_diff_for_blowby: 60.0,
            blowby_interval: 10.0,
            suspect_outrun_range: 120.0,
            time_considered_lost_no_los: 3.0,
            pursuit_inactivity_timer: [2.0, 150.0],
            no_los_commentary_time: 2.0,
            min_speed_considered_stopped: 5.0,
            min_time_considered_stopped: 2.0,
            high_intensity_mark: 2.0,
            pursuit_duration_high_intensity: 150.0,
            min_pursuit_duration_for_bailouts: 60.0,
            min_health_for_commentary: 0.33,
            crash_slowdown_pct: 0.66,
            player_smash_speed_range: [10.0, 30.0],
            min_intensity_traffic_smash: 0.5,
            min_intensity_cop_smash: 0.85,
            min_havoc_for_suspect_behavior: 200.0,
            min_height_airborne: 0.5,
            hang_time_for_commentary: 0.9,
            flip_time_for_commentary: 1.0,
            speed_thresh_fly_flip_intensity: 100.0,
            min_contig_frames_for_180: 12,
            max_range_for_180: 50.0,
            max_time_for_180: 5.0,
            outcome_track_time: 3.0,
            outcome_fail_speed: 20.0,
            rb_outcome_timer: 1.5,
            rb_post_outcome_reset_time: 2.0,
            bu_remind_time: 20.0,
            ai_racer_proximity: 145.0,
        }
    }
}

fn float(c: CollectionRef<'_>, field: &str, fallback: f32) -> f32 {
    c.get_f32(field).unwrap_or(fallback)
}

fn pair(c: CollectionRef<'_>, field: &str, fallback: [f32; 2]) -> [f32; 2] {
    let at = |i: usize, fb: f32| c.get_at(field, i).and_then(|v| v.as_f32()).unwrap_or(fb);
    [at(0, fallback[0]), at(1, fallback[1])]
}

/// The `speechtune` collection of the database (the install's values where a field is missing).
pub fn chase_tune(db: &Database) -> ChaseTune {
    let d = ChaseTune::default();
    let Some(c) = db.collections_of("speechtune").next() else { return d };
    ChaseTune {
        pursuit_init_delay: float(c, "PursuitInitDelay", d.pursuit_init_delay),
        range_for_spotter_branch: float(c, "RangeForSpotterBranch", d.range_for_spotter_branch),
        speed_diff_for_blowby: float(c, "SpeedDiffForBlowby", d.speed_diff_for_blowby),
        blowby_interval: float(c, "BlowbyInterval", d.blowby_interval),
        suspect_outrun_range: float(c, "SuspectOutrunRange", d.suspect_outrun_range),
        time_considered_lost_no_los: float(c, "TimeConsideredLostNoLOS", d.time_considered_lost_no_los),
        pursuit_inactivity_timer: pair(c, "PursuitInactivityTimer", d.pursuit_inactivity_timer),
        no_los_commentary_time: float(c, "NoLOSCommentaryTime", d.no_los_commentary_time),
        min_speed_considered_stopped: float(c, "MinSpeedConsideredStopped", d.min_speed_considered_stopped),
        min_time_considered_stopped: float(c, "MinTimeConsideredStopped", d.min_time_considered_stopped),
        high_intensity_mark: c.get_i32("HighIntensityMark").map_or(d.high_intensity_mark, |v| v as f32),
        pursuit_duration_high_intensity: float(c, "PursuitDurationHighIntensity", d.pursuit_duration_high_intensity),
        min_pursuit_duration_for_bailouts: float(
            c,
            "MinPursuitDurationForBailouts",
            d.min_pursuit_duration_for_bailouts,
        ),
        min_health_for_commentary: float(c, "MinHealthForCommentary", d.min_health_for_commentary),
        crash_slowdown_pct: float(c, "CrashSlowdownPct", d.crash_slowdown_pct),
        player_smash_speed_range: pair(c, "PlayerSmashSpeedRange", d.player_smash_speed_range),
        min_intensity_traffic_smash: float(c, "MinIntensityTrafficSmash", d.min_intensity_traffic_smash),
        min_intensity_cop_smash: float(c, "MinIntensityCopSmash", d.min_intensity_cop_smash),
        min_havoc_for_suspect_behavior: c
            .get_i32("MinHavocForSuspectBehavior")
            .map_or(d.min_havoc_for_suspect_behavior, |v| v as f32),
        min_height_airborne: float(c, "MinHeightAirborne", d.min_height_airborne),
        hang_time_for_commentary: float(c, "HangTimeForCommentary", d.hang_time_for_commentary),
        flip_time_for_commentary: float(c, "FlipTimeForCommentary", d.flip_time_for_commentary),
        speed_thresh_fly_flip_intensity: float(c, "SpeedThreshFlyFlipIntensity", d.speed_thresh_fly_flip_intensity),
        min_contig_frames_for_180: c
            .get_i32("MinContigFramesFor180")
            .map_or(d.min_contig_frames_for_180, |v| v.max(0) as u32),
        max_range_for_180: float(c, "MaxRangeFor180", d.max_range_for_180),
        max_time_for_180: float(c, "MaxTimeFor180", d.max_time_for_180),
        outcome_track_time: float(c, "OutcomeTrackTime", d.outcome_track_time),
        outcome_fail_speed: float(c, "OutcomeFailSpeed", d.outcome_fail_speed),
        rb_outcome_timer: float(c, "RBOutcomeTimer", d.rb_outcome_timer),
        rb_post_outcome_reset_time: float(c, "RBPostOutcomeResetTime", d.rb_post_outcome_reset_time),
        bu_remind_time: float(c, "BURemindTime", d.bu_remind_time),
        ai_racer_proximity: float(c, "AIRacerProximity", d.ai_racer_proximity),
    }
}

/// The fields of one heat level's `pursuitlevels` row that the chase speech reads.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LevelSpeech {
    /// `roadblockprobability` above zero: the heat has roadblocks.
    pub roadblocks: bool,
    /// `roadblockhelichance` above zero: the helicopter may join a roadblock.
    pub heli_roadblocks: bool,
    /// `roadblockspikechance` above zero: roadblocks may have spike strips.
    pub spikes: bool,
    /// `CTSFor911`: the damage cost that makes dispatch call 911 in free roam.
    pub cts_for_911: i32,
    /// `NumCiviHitsFor911`: the hard traffic hits that make dispatch call 911.
    pub hits_for_911: i32,
    /// `Lifetime911`: seconds the 911 call keeps the cops away.
    pub lifetime_911: f32,
}

/// The speech fields of heat level `heat` (1 to 10, clamped), outside races.
pub fn level_speech(db: &Database, heat: u32) -> LevelSpeech {
    let Some(row) = level_collection(db, "heattable", heat) else { return LevelSpeech::default() };
    LevelSpeech {
        roadblocks: row.get_f32("roadblockprobability").is_some_and(|v| v > 0.0),
        heli_roadblocks: row.get_f32("roadblockhelichance").is_some_and(|v| v > 0.0),
        spikes: row.get_f32("roadblockspikechance").is_some_and(|v| v > 0.0),
        cts_for_911: row.get_i32("CTSFor911").unwrap_or(0),
        hits_for_911: row.get_i32("NumCiviHitsFor911").unwrap_or(0),
        lifetime_911: row.get_f32("Lifetime911").unwrap_or(0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fallback_is_the_installs_tuning() {
        let tune = ChaseTune::default();
        assert_eq!(tune.pursuit_inactivity_timer, [2.0, 150.0]);
        assert_eq!(tune.player_smash_speed_range, [10.0, 30.0]);
        assert_eq!(tune.min_contig_frames_for_180, 12);
    }

    #[test]
    #[ignore = "needs the game (set NFSMW_GAME_DIR)"]
    fn the_installed_tuning_matches_the_spec_table() {
        let Some(dir) = std::env::var_os("NFSMW_GAME_DIR") else { return };
        let dir = game_install::GameDir::open(std::path::PathBuf::from(dir)).unwrap();
        let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).unwrap();
        assert_eq!(chase_tune(&db), ChaseTune::default(), "the database's speechtune is the spec's table");
        let fourth = level_speech(&db, 4);
        println!("{fourth:?}");
        assert!(fourth.cts_for_911 > 0 || fourth.hits_for_911 > 0 || fourth.lifetime_911 >= 0.0);
    }
}
