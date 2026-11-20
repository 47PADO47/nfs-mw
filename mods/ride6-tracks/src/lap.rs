//! Lap timing on a track's spline: where the player is along the lap, when the start line is crossed, how long each lap took.
//!
//! The lap is the track spline sampled about every 2 m and turned to start at the first slot of the starting grid (the
//! importer writes it as `worlds/<id>.lap`, [`Track::to_text`]). The player's position is matched to the nearest sample near
//! the last one, which keeps a track that passes close to itself (a hairpin, a pit lane beside the straight) from confusing
//! the match. A lap counts when the start line is crossed forward after at least nine tenths of the lap were covered.

use anyhow::{Context, Result, bail, ensure};

/// How far from the line of the track the player may be and still be on it, metres. Further away, nothing is measured.
const MAX_LATERAL: f32 = 80.0;
/// How many samples ahead or behind the last match are searched first (200 samples are 400 m).
const WINDOW: usize = 200;
/// The share of the lap that must be covered for a crossing of the start line to be a lap.
const LAP_SHARE: f32 = 0.9;

/// The lap as a closed polyline in render space (x, y), metres.
#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    points: Vec<[f32; 2]>,
    /// The distance from the start to each sample, metres.
    along: Vec<f32>,
    length: f32,
}

impl Track {
    /// A track from the samples of a closed lap. At least 8 samples, all finite.
    pub fn new(points: Vec<[f32; 2]>) -> Result<Self> {
        ensure!(points.len() >= 8, "a lap needs at least 8 samples, this has {}", points.len());
        ensure!(points.iter().flatten().all(|v| v.is_finite()), "a lap sample is not finite");
        let mut along = Vec::with_capacity(points.len());
        let mut length = 0.0;
        for (i, point) in points.iter().enumerate() {
            along.push(length);
            length += distance(*point, points[(i + 1) % points.len()]);
        }
        ensure!(length > 10.0, "the lap is {length} m long");
        Ok(Self { points, along, length })
    }

    /// The same lap with its start line `ahead` metres along from the sample nearest `start`: the first slot of the starting grid
    /// stands behind the line, so the first lap is timed from the grid.
    pub fn starting_after(mut self, start: [f32; 2], ahead: f32) -> Self {
        let count = self.points.len();
        let nearest = (0..count)
            .min_by(|&a, &b| distance(self.points[a], start).total_cmp(&distance(self.points[b], start)))
            .unwrap_or(0);
        let spacing = self.length / count as f32;
        let at = (nearest + (ahead / spacing).round() as usize) % count;
        self.points.rotate_left(at);
        Self::new(self.points).expect("a rotated lap is still a lap")
    }

    pub fn length(&self) -> f32 {
        self.length
    }

    /// The point `distance` metres along the lap from its first sample (the start line), the lap closing on itself.
    pub fn at(&self, distance: f32) -> [f32; 2] {
        let distance = distance.rem_euclid(self.length);
        let next = self.along.partition_point(|&along| along <= distance);
        let (from, to) = (next.saturating_sub(1), next % self.points.len());
        let span = if to == 0 { self.length - self.along[from] } else { self.along[to] - self.along[from] };
        let t = if span > 0.0 { (distance - self.along[from]) / span } else { 0.0 };
        let (a, b) = (self.points[from], self.points[to]);
        [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
    }

    pub fn len(&self) -> usize {
        self.points.len()
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// The text of the lap file: one `x y` pair per line.
    pub fn to_text(&self) -> String {
        self.points.iter().map(|[x, y]| format!("{x:.2} {y:.2}\n")).collect()
    }

    /// A lap from the text [`Track::to_text`] writes.
    pub fn from_text(text: &str) -> Result<Self> {
        let mut points = Vec::new();
        for (number, line) in text.lines().enumerate().filter(|(_, line)| !line.trim().is_empty()) {
            let mut fields = line.split_whitespace().map(str::parse::<f32>);
            let (Some(x), Some(y), None) = (fields.next(), fields.next(), fields.next()) else {
                bail!("line {} is not two numbers", number + 1);
            };
            points.push([
                x.with_context(|| format!("line {}", number + 1))?,
                y.with_context(|| format!("line {}", number + 1))?,
            ]);
        }
        Self::new(points)
    }

    /// The sample nearest `at` among those within [`WINDOW`] of `near` (all of them without a `near`), and its distance.
    fn nearest(&self, at: [f32; 2], near: Option<usize>) -> (usize, f32) {
        let count = self.points.len();
        let candidates: Box<dyn Iterator<Item = usize>> = match near {
            Some(near) => Box::new((0..=2 * WINDOW).map(move |k| (near + count - WINDOW % count + k) % count)),
            None => Box::new(0..count),
        };
        candidates
            .map(|i| (i, distance(self.points[i], at)))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap_or((0, f32::INFINITY))
    }
}

fn distance(a: [f32; 2], b: [f32; 2]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

/// A virtual car for checking the lap timer in the game without driving a lap: it runs the lap at a constant speed, starting a
/// few metres before the start line, so the messages of the start line and of each lap appear on the HUD
/// (`RIDE6_TRACKS_LAP_DEMO=<metres per second>`, `mods/ride6-tracks/README.md`).
#[derive(Debug, Clone, Copy)]
pub struct Demo {
    speed: f32,
    distance: f32,
}

impl Demo {
    /// A run at `speed` m/s on `track`, `lead` metres before the start line.
    pub fn new(track: &Track, speed: f32, lead: f32) -> Self {
        Self { speed, distance: track.length() - lead }
    }

    /// The car's position after `dt` more seconds (render space).
    pub fn step(&mut self, track: &Track, dt: f32) -> [f32; 3] {
        self.distance += self.speed * dt;
        let [x, y] = track.at(self.distance);
        [x, y, 0.0]
    }
}

/// What a step of the timer found.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    /// The start line was crossed with no lap running: the timer starts.
    Started,
    /// A lap is complete: its time and whether it is the best so far.
    Lap { number: u32, seconds: f32, best: bool },
}

/// The lap timer of one player on one track.
#[derive(Debug, Clone)]
pub struct Timer {
    track: Track,
    /// The sample matched at the last step.
    last: Option<usize>,
    /// The distance covered since the start line, metres (forward progress only).
    covered: f32,
    /// Seconds of the lap in progress, `None` before the first crossing of the start line.
    running: Option<f32>,
    laps: u32,
    best: Option<f32>,
}

impl Timer {
    pub fn new(track: Track) -> Self {
        Self { track, last: None, covered: 0.0, running: None, laps: 0, best: None }
    }

    /// The lap being timed.
    pub fn track(&self) -> &Track {
        &self.track
    }

    /// The best lap so far, seconds.
    pub fn best(&self) -> Option<f32> {
        self.best
    }

    /// The lap in progress, seconds, once the start line has been crossed.
    pub fn current(&self) -> Option<f32> {
        self.running
    }

    /// Laps completed.
    pub fn laps(&self) -> u32 {
        self.laps
    }

    /// Moves the timer on by `dt` seconds with the player at `position` (render space). Returns what happened, if anything.
    pub fn update(&mut self, position: [f32; 3], dt: f32) -> Option<Event> {
        if let Some(seconds) = self.running.as_mut() {
            *seconds += dt.max(0.0);
        }
        let (index, lateral) = self.track.nearest([position[0], position[1]], self.last);
        if lateral > MAX_LATERAL {
            self.last = None;
            return None;
        }
        let count = self.track.len();
        let previous = self.last.replace(index)?;
        // A step along the lap, forward or back, the short way round.
        let step = (index + count - previous) % count;
        let forward = step < count / 2;
        let moved = if forward { step } else { count - step };
        let metres = moved as f32 * self.track.length() / count as f32;
        if !forward {
            self.covered = (self.covered - metres).max(0.0);
            return None;
        }
        self.covered += metres;
        let crossed = previous + step >= count;
        if !crossed {
            return None;
        }
        self.crossed_the_line()
    }

    fn crossed_the_line(&mut self) -> Option<Event> {
        let complete = self.running.is_some() && self.covered >= LAP_SHARE * self.track.length();
        self.covered = 0.0;
        if !complete {
            let first = self.running.is_none();
            self.running = Some(0.0);
            return first.then_some(Event::Started);
        }
        let seconds = self.running.replace(0.0).unwrap_or_default();
        self.laps += 1;
        let best = self.best.is_none_or(|best| seconds < best);
        if best {
            self.best = Some(seconds);
        }
        Some(Event::Lap { number: self.laps, seconds, best })
    }
}

/// A lap time as `m:ss.cc`.
pub fn clock(seconds: f32) -> String {
    let hundredths = (seconds * 100.0).round() as u32;
    format!("{}:{:02}.{:02}", hundredths / 6000, hundredths / 100 % 60, hundredths % 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A circle of radius 100 m with 200 samples, counter-clockwise from (100, 0).
    fn circle() -> Track {
        let points = (0..200)
            .map(|i| {
                let a = i as f32 / 200.0 * std::f32::consts::TAU;
                [100.0 * a.cos(), 100.0 * a.sin()]
            })
            .collect();
        Track::new(points).unwrap()
    }

    fn on_circle(angle: f32, radius: f32) -> [f32; 3] {
        [radius * angle.cos(), radius * angle.sin(), 0.0]
    }

    /// Drives `laps` laps of the circle at 20 m/s, 0.05 s steps, and returns the events.
    fn drive(timer: &mut Timer, laps: f32, radius: f32) -> Vec<Event> {
        let mut events = Vec::new();
        let speed = 20.0;
        let total = laps * std::f32::consts::TAU * 100.0 / speed;
        let steps = (total / 0.05) as usize;
        // Start a little before the line, so the first crossing is seen.
        let first = -0.2_f32;
        for k in 0..steps {
            let angle = first + (k as f32 * 0.05 * speed) / 100.0;
            events.extend(timer.update(on_circle(angle, radius), 0.05));
        }
        events
    }

    #[test]
    fn the_start_line_starts_the_clock_and_each_lap_is_timed() {
        let mut timer = Timer::new(circle());
        let events = drive(&mut timer, 2.05, 100.0);
        assert_eq!(events[0], Event::Started);
        let laps: Vec<_> = events
            .iter()
            .filter_map(|e| if let Event::Lap { seconds, .. } = e { Some(*seconds) } else { None })
            .collect();
        assert_eq!(laps.len(), 2, "{events:?}");
        for seconds in &laps {
            assert!((seconds - 31.4).abs() < 0.3, "a lap of 628 m at 20 m/s is {seconds}");
        }
        assert_eq!(timer.laps(), 2);
        assert!(timer.best().is_some());
    }

    #[test]
    fn a_player_off_the_track_measures_nothing_and_going_back_over_the_line_is_not_a_lap() {
        let mut timer = Timer::new(circle());
        assert_eq!(timer.update([1000.0, 1000.0, 0.0], 0.1), None);
        assert_eq!(timer.update(on_circle(-0.2, 100.0), 0.1), None);
        assert_eq!(timer.update(on_circle(0.1, 100.0), 0.1), Some(Event::Started));
        // Back over the line: no lap.
        assert_eq!(timer.update(on_circle(-0.1, 100.0), 0.1), None);
        assert_eq!(timer.laps(), 0);
    }

    #[test]
    fn a_short_cut_across_the_circle_is_not_a_lap() {
        let mut timer = Timer::new(circle());
        timer.update(on_circle(-0.2, 100.0), 0.1);
        assert_eq!(timer.update(on_circle(0.1, 100.0), 0.1), Some(Event::Started));
        // Jump to the far side and back to the line: under nine tenths covered through steps within the window.
        for angle in [0.5, 1.0, 1.5, 2.0] {
            timer.update(on_circle(angle, 100.0), 0.1);
        }
        for angle in [1.0, 0.5, 0.1, -0.1, 0.1] {
            timer.update(on_circle(angle, 100.0), 0.1);
        }
        assert_eq!(timer.laps(), 0);
    }

    #[test]
    fn the_file_text_round_trips_and_a_lap_can_start_elsewhere() {
        let track = circle();
        let again = Track::from_text(&track.to_text()).unwrap();
        assert_eq!(again.len(), 200);
        assert!((again.length() - track.length()).abs() < 0.5);
        let moved = track.clone().starting_after([-100.0, 0.0], 0.0);
        assert!(distance(moved.points[0], [-100.0, 0.0]) < 1.0);
        // 20 m on from (100, 0), counter-clockwise: the sample about 20 m up the circle.
        let ahead = track.starting_after([100.0, 0.0], 20.0);
        assert!((ahead.points[0][1] - 20.0).abs() < 2.5, "{:?}", ahead.points[0]);
        assert!(Track::from_text("1 2\n3\n").is_err());
        assert!(Track::from_text("1 2\n").is_err(), "too few samples");
    }

    #[test]
    fn a_point_along_the_lap_wraps_and_a_demo_run_makes_the_start_and_every_lap() {
        let track = circle();
        let [x, y] = track.at(0.0);
        assert!((x - 100.0).abs() < 0.01 && y.abs() < 0.01);
        let [x, y] = track.at(track.length() / 4.0);
        assert!(x.abs() < 1.0 && (y - 100.0).abs() < 1.0, "a quarter of the way round: {x} {y}");
        assert_eq!(track.at(track.length() + 10.0), track.at(10.0));
        let mut timer = Timer::new(circle());
        let mut demo = Demo::new(timer.track(), 50.0, 20.0);
        let mut events = Vec::new();
        for _ in 0..(3 * 628 * 20 / 50 + 40) {
            let position = demo.step(&circle(), 0.05);
            events.extend(timer.update(position, 0.05));
        }
        assert_eq!(events[0], Event::Started);
        let laps = events.iter().filter(|event| matches!(event, Event::Lap { .. })).count();
        assert_eq!(laps, 3, "{events:?}");
    }

    #[test]
    fn lap_times_read_as_minutes_seconds_and_hundredths() {
        assert_eq!(clock(92.456), "1:32.46");
        assert_eq!(clock(5.0), "0:05.00");
        assert_eq!(clock(600.0), "10:00.00");
    }
}
