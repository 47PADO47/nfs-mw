//! What a racer's run adds up to for the post-race pages: the top speed, the distance driven, the time and place at
//! 20, 40, 60 and 80 percent of the course, and the two drag marks. Spec: `docs/specs/race-results.md`.

/// The shares of the course at which a racer's time and place are kept (`docs/provenance/race-results-screens.md`).
pub const STAGE_PERCENTS: [f32; 4] = [20.0, 40.0, 60.0, 80.0];
/// A quarter mile in metres: the distance the drag race's `Quarter Mile Time` is read at.
pub const QUARTER_MILE: f32 = 402.335;
/// 60 mph in m/s: the speed the drag race's `0 to 60 mph` is read at.
pub const SIXTY_MPH: f32 = 26.8218;

/// The time and the place a racer had when it passed a stage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StageMark {
    /// Race seconds.
    pub time: f32,
    /// 1 for first.
    pub place: u32,
}

/// One racer's totals.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RacerStats {
    /// The highest speed seen, m/s.
    pub top_speed: f32,
    /// Metres driven while the racer was racing.
    pub distance: f32,
    /// The marks at [`STAGE_PERCENTS`].
    pub stages: [Option<StageMark>; 4],
    /// Race seconds when the speed first reached [`SIXTY_MPH`].
    pub sixty_at: Option<f32>,
    /// Race seconds when the distance first reached [`QUARTER_MILE`].
    pub quarter_at: Option<f32>,
}

impl RacerStats {
    /// The stage a racer at `percent` records now: the highest share it has passed whose mark is still empty. One stage
    /// per call, as the original records one per frame: a racer that jumps past two shares fills the higher first.
    pub fn stage_due(&self, percent: f32) -> Option<usize> {
        (0..STAGE_PERCENTS.len()).rev().find(|&i| percent >= STAGE_PERCENTS[i] && self.stages[i].is_none())
    }

    /// Keeps a speed seen at race time `time`: the top speed and the 0 to 60 mark.
    pub fn note_speed(&mut self, mps: f32, time: f32) {
        if mps > self.top_speed {
            self.top_speed = mps;
        }
        if self.sixty_at.is_none() && self.top_speed >= SIXTY_MPH {
            self.sixty_at = Some(time);
        }
    }

    /// Adds `metres` driven, at race time `time`: the distance and the quarter mile mark.
    pub fn note_distance(&mut self, metres: f32, time: f32) {
        self.distance += metres;
        if self.quarter_at.is_none() && self.distance >= QUARTER_MILE {
            self.quarter_at = Some(time);
        }
    }

    /// The average speed over `secs` of racing, m/s (0 for no time).
    pub fn average_speed(&self, secs: f32) -> f32 {
        if secs <= 0.0 {
            return 0.0;
        }
        self.distance / secs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stage_is_due_once_its_share_is_passed() {
        let mut stats = RacerStats::default();
        assert_eq!(stats.stage_due(19.9), None);
        assert_eq!(stats.stage_due(20.0), Some(0));
        stats.stages[0] = Some(StageMark { time: 30.0, place: 1 });
        assert_eq!(stats.stage_due(25.0), None);
        assert_eq!(stats.stage_due(100.0), Some(3));
    }

    #[test]
    fn a_jump_fills_the_highest_stage_first() {
        let mut stats = RacerStats::default();
        assert_eq!(stats.stage_due(45.0), Some(1));
        stats.stages[1] = Some(StageMark { time: 1.0, place: 2 });
        assert_eq!(stats.stage_due(45.0), Some(0), "the lower stage follows on the next frame");
    }

    #[test]
    fn speed_and_distance_keep_their_marks() {
        let mut stats = RacerStats::default();
        stats.note_speed(20.0, 2.0);
        stats.note_speed(30.0, 3.5);
        stats.note_speed(25.0, 4.0);
        assert_eq!((stats.top_speed, stats.sixty_at), (30.0, Some(3.5)));
        stats.note_distance(400.0, 9.0);
        assert_eq!(stats.quarter_at, None);
        stats.note_distance(5.0, 9.1);
        stats.note_distance(5.0, 9.2);
        assert_eq!(stats.quarter_at, Some(9.1));
        assert_eq!(stats.average_speed(10.0), 41.0);
        assert_eq!(stats.average_speed(0.0), 0.0);
    }
}
