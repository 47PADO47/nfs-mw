//! The course of a race and a racer's progress along it: checkpoints in order, the finish line, laps.

use glam::Vec3;

use crate::gate::Gate;

/// What a racer's movement raised.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// Checkpoint `n` (numbered from 1) was reached.
    Checkpoint(usize),
    /// Lap `n` (numbered from 1) was finished and another follows.
    Lap(u32),
    /// The last lap was finished.
    Finished,
}

/// The route: where the racers start, the checkpoints in order, the finish line and the number of laps.
#[derive(Debug, Clone, PartialEq)]
pub struct Course {
    /// The front of the grid. Only the percent complete uses it.
    pub start: Vec3,
    pub checkpoints: Vec<Gate>,
    /// The finish line. Without one, the last checkpoint finishes the lap.
    pub finish: Option<Gate>,
    pub laps: u32,
}

impl Course {
    /// The route as points: the start, the checkpoint centres and the finish centre.
    fn route(&self) -> Vec<Vec3> {
        let mut points = vec![self.start];
        points.extend(self.checkpoints.iter().map(|g| g.centre));
        if let Some(finish) = &self.finish {
            points.push(finish.centre);
        }
        points
    }

    /// Length of one lap along the route, metres (straight lines between the points).
    pub fn lap_length(&self) -> f32 {
        let points = self.route();
        points.windows(2).map(|pair| pair[0].distance(pair[1])).sum()
    }

    /// Length of the whole race, metres.
    pub fn length(&self) -> f32 {
        self.lap_length() * self.laps.max(1) as f32
    }

    /// Percent complete (0 to 100) of a racer with `progress` standing at `position`: the laps done, the checkpoints
    /// reached and the share of the leg to the next gate. 100 once finished. **[unconfirmed]** (the original writes
    /// this number in code that was not read).
    pub fn percent(&self, progress: &Progress, position: Vec3) -> f32 {
        if progress.finished {
            return 100.0;
        }
        let points = self.route();
        let total = self.lap_length();
        if total <= 0.0 {
            return 0.0;
        }
        // The racer has completed `reached` legs of this lap: the legs start -> cp1 -> ... Leg `reached` is the one it
        // is on, from point `reached` to point `reached + 1`.
        let reached = progress.reached.min(points.len() - 1);
        let done: f32 = points[..=reached].windows(2).map(|pair| pair[0].distance(pair[1])).sum();
        let leg = match points.get(reached + 1) {
            Some(next) => {
                let from = points[reached];
                let length = from.distance(*next);
                let along = (position - from).dot((*next - from).normalize_or_zero());
                along.clamp(0.0, length)
            }
            None => 0.0,
        };
        let lap = ((done + leg) / total).clamp(0.0, 1.0);
        (100.0 * (progress.laps as f32 + lap) / self.laps.max(1) as f32).clamp(0.0, 100.0)
    }
}

/// How far one racer has got: the laps finished, the checkpoints reached this lap and whether it has finished.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Progress {
    /// Laps completed.
    pub laps: u32,
    /// Checkpoints reached this lap (the next expected is `reached + 1`).
    pub reached: usize,
    pub finished: bool,
}

impl Progress {
    /// Moves the racer from `from` to `to` and appends the events that raised. Checkpoints count in order only: a
    /// later gate crossed early is ignored (`docs/specs/race-mode.md` section 3).
    pub fn advance(&mut self, course: &Course, from: Vec3, to: Vec3, out: &mut Vec<Event>) {
        if self.finished {
            return;
        }
        if let Some(gate) = course.checkpoints.get(self.reached) {
            if !gate.touches(from, to) {
                return;
            }
            self.reached += 1;
            out.push(Event::Checkpoint(self.reached));
            // Without a finish line the last checkpoint closes the lap.
            if self.reached == course.checkpoints.len() && course.finish.is_none() {
                self.close_lap(course, out);
            }
            return;
        }
        let Some(finish) = &course.finish else { return };
        if finish.touches(from, to) {
            self.close_lap(course, out);
        }
    }

    fn close_lap(&mut self, course: &Course, out: &mut Vec<Event>) {
        self.laps += 1;
        self.reached = 0;
        if self.laps >= course.laps.max(1) {
            self.finished = true;
            out.push(Event::Finished);
            return;
        }
        out.push(Event::Lap(self.laps));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gate(z: f32) -> Gate {
        Gate::new(Vec3::new(0.0, 0.0, z), Vec3::Z, Vec3::Y, [2.0, 40.0, 10.0])
    }

    fn sprint() -> Course {
        Course { start: Vec3::ZERO, checkpoints: vec![gate(100.0), gate(200.0)], finish: Some(gate(300.0)), laps: 1 }
    }

    fn drive(course: &Course, progress: &mut Progress, zs: &[f32]) -> Vec<Event> {
        let mut events = Vec::new();
        for pair in zs.windows(2) {
            progress.advance(course, Vec3::new(0.0, 0.0, pair[0]), Vec3::new(0.0, 0.0, pair[1]), &mut events);
        }
        events
    }

    #[test]
    fn a_sprint_reports_checkpoints_in_order_then_the_finish() {
        let course = sprint();
        let mut progress = Progress::default();
        let events = drive(&course, &mut progress, &[0.0, 50.0, 150.0, 250.0, 350.0]);
        assert_eq!(events, [Event::Checkpoint(1), Event::Checkpoint(2), Event::Finished]);
        assert!(progress.finished);
    }

    #[test]
    fn a_checkpoint_out_of_order_does_not_count() {
        let course = sprint();
        let mut progress = Progress::default();
        // Straight from the start past checkpoint 2's z: but checkpoint 1 lies on the way, so use a side step.
        let mut events = Vec::new();
        progress.advance(&course, Vec3::new(100.0, 0.0, 190.0), Vec3::new(100.0, 0.0, 210.0), &mut events);
        assert!(events.is_empty());
        assert_eq!(progress.reached, 0);
        // A finish line crossed before the checkpoints is also ignored.
        progress.advance(&course, Vec3::new(0.0, 0.0, 290.0), Vec3::new(0.0, 0.0, 310.0), &mut events);
        assert!(events.is_empty() && !progress.finished);
    }

    #[test]
    fn a_circuit_counts_laps_and_resets_the_checkpoints() {
        let course = Course { laps: 3, ..sprint() };
        let mut progress = Progress::default();
        let lap = [0.0, 50.0, 150.0, 250.0, 350.0];
        let events = drive(&course, &mut progress, &lap);
        assert_eq!(events, [Event::Checkpoint(1), Event::Checkpoint(2), Event::Lap(1)]);
        assert_eq!((progress.laps, progress.reached), (1, 0));
        drive(&course, &mut progress, &lap);
        let last = drive(&course, &mut progress, &lap);
        assert_eq!(last.last(), Some(&Event::Finished));
        assert_eq!(progress.laps, 3);
    }

    #[test]
    fn without_a_finish_line_the_last_checkpoint_finishes() {
        let course = Course { finish: None, ..sprint() };
        let mut progress = Progress::default();
        let events = drive(&course, &mut progress, &[0.0, 150.0, 250.0]);
        assert_eq!(events, [Event::Checkpoint(1), Event::Checkpoint(2), Event::Finished]);
    }

    #[test]
    fn a_finished_racer_raises_nothing_more() {
        let course = sprint();
        let mut progress = Progress::default();
        drive(&course, &mut progress, &[0.0, 150.0, 250.0, 350.0]);
        assert!(drive(&course, &mut progress, &[350.0, 0.0, 350.0]).is_empty());
    }

    #[test]
    fn percent_rises_along_the_route_and_is_100_when_finished() {
        let course = sprint();
        let mut progress = Progress::default();
        assert_eq!(course.percent(&progress, Vec3::ZERO), 0.0);
        let skipped = course.percent(&progress, Vec3::new(0.0, 0.0, 150.0));
        assert!((skipped - 100.0 / 3.0).abs() < 1e-3, "a leg is capped at its end: {skipped}");
        progress.reached = 1;
        let later = course.percent(&progress, Vec3::new(0.0, 0.0, 150.0));
        assert!((later - 50.0).abs() < 1e-3, "{later}");
        progress.finished = true;
        assert_eq!(course.percent(&progress, Vec3::ZERO), 100.0);
    }

    #[test]
    fn percent_counts_laps_of_a_circuit() {
        let course = Course { laps: 2, ..sprint() };
        let progress = Progress { laps: 1, reached: 0, finished: false };
        let at_start = course.percent(&progress, Vec3::ZERO);
        assert!((at_start - 50.0).abs() < 1e-3, "{at_start}");
    }
}
