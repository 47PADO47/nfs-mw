//! One race: the course, the clock and the racers, with the events each tick raises.

use glam::Vec3;

use crate::clock::{Clock, ClockEvent, Phase};
use crate::course::{Course, Event, Progress};
use crate::knockout::knockout_due;
use crate::points::{KMH_TO_MS, SpeedTraps, overtime_loss};
use crate::ranking::{Standing, places};
use crate::stats::{RacerStats, StageMark};

/// What the race needs besides the course.
#[derive(Debug, Clone, PartialEq)]
pub struct RaceConfig {
    /// Racers dropped at the end of each lap (0 for none).
    pub knockouts_per_lap: u32,
    /// Rank by speed-trap points.
    pub rank_by_points: bool,
    /// Time limit, seconds (0 for none).
    pub time_limit: f32,
    /// Length of the countdown board, seconds.
    pub countdown: f32,
    /// How many speed traps the race has.
    pub speed_traps: usize,
    /// Points lost per second past the time limit (speed-trap races).
    pub overtime_penalty: f32,
}

impl Default for RaceConfig {
    fn default() -> Self {
        Self {
            knockouts_per_lap: 0,
            rank_by_points: false,
            time_limit: 0.0,
            countdown: 3.0,
            speed_traps: 0,
            overtime_penalty: 0.0,
        }
    }
}

/// One racer of the race.
#[derive(Debug, Clone, PartialEq)]
pub struct Racer {
    pub human: bool,
    pub progress: Progress,
    pub position: Vec3,
    /// Percent complete, 0 to 100.
    pub percent: f32,
    pub finished_at: Option<f32>,
    pub out_at: Option<f32>,
    /// The racer's speed, m/s, as the caller last told it ([`Race::set_speed`]).
    pub speed: f32,
    pub traps: SpeedTraps,
    /// The race time of each checkpoint reached, this lap and earlier.
    pub checkpoint_times: Vec<f32>,
    /// The race time at the end of each lap.
    pub lap_times: Vec<f32>,
    /// The totals for the post-race pages.
    pub stats: RacerStats,
}

impl Racer {
    fn new(human: bool, position: Vec3, traps: usize) -> Self {
        Self {
            human,
            progress: Progress::default(),
            position,
            percent: 0.0,
            finished_at: None,
            out_at: None,
            speed: 0.0,
            traps: SpeedTraps::new(traps),
            checkpoint_times: Vec::new(),
            lap_times: Vec::new(),
            stats: RacerStats::default(),
        }
    }

    /// The racer still counts in the race: it has not finished and is not out.
    pub fn racing(&self) -> bool {
        self.finished_at.is_none() && self.out_at.is_none()
    }

    /// The racer is done: finished or out.
    pub fn done(&self) -> bool {
        !self.racing()
    }
}

/// An event of the race, in the order it happened.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RaceEvent {
    Count(u32),
    Go,
    TimeUp,
    Checkpoint {
        racer: usize,
        number: usize,
    },
    /// A human racer's checkpoint added seconds to the clock.
    Bonus {
        racer: usize,
        seconds: f32,
    },
    /// A racer passed speed trap `slot` fast enough to pay, at `kmh`.
    SpeedTrap {
        racer: usize,
        slot: usize,
        kmh: f32,
    },
    Lap {
        racer: usize,
        lap: u32,
    },
    Finished {
        racer: usize,
        time: f32,
    },
    KnockedOut {
        racer: usize,
    },
    /// Every human racer is done (finished or out), or the time ran out: the race is over.
    Over,
}

/// How one racer ended.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Outcome {
    pub racer: usize,
    /// 1 for first.
    pub place: u32,
    /// The finish time, when the racer finished.
    pub time: Option<f32>,
    pub finished: bool,
}

pub struct Race {
    course: Course,
    config: RaceConfig,
    clock: Clock,
    racers: Vec<Racer>,
    over: bool,
}

impl Race {
    /// A race on the grid. `grid` has one `(is_human, position)` per racer.
    pub fn new(course: Course, config: RaceConfig, grid: &[(bool, Vec3)]) -> Self {
        let mut config = config;
        config.speed_traps = course.traps.len().max(config.speed_traps);
        let clock = Clock::new(config.time_limit, config.countdown);
        let racers = grid.iter().map(|&(human, at)| Racer::new(human, at, config.speed_traps)).collect();
        Self { course, config, clock, racers, over: false }
    }

    pub fn course(&self) -> &Course {
        &self.course
    }

    pub fn config(&self) -> &RaceConfig {
        &self.config
    }

    pub fn phase(&self) -> Phase {
        self.clock.phase()
    }

    pub fn elapsed(&self) -> f32 {
        self.clock.elapsed()
    }

    pub fn time_left(&self) -> f32 {
        self.clock.time_left()
    }

    pub fn has_time_limit(&self) -> bool {
        self.clock.has_limit()
    }

    pub fn racers(&self) -> &[Racer] {
        &self.racers
    }

    pub fn is_over(&self) -> bool {
        self.over
    }

    /// Starts the countdown (the cars stay held until `Go`).
    pub fn begin_countdown(&mut self) -> Vec<RaceEvent> {
        let mut raised = Vec::new();
        self.clock.begin_countdown(&mut raised);
        raised.into_iter().map(from_clock).collect()
    }

    /// Starts the race at once (a rolling start).
    pub fn start_now(&mut self) -> Vec<RaceEvent> {
        let mut raised = Vec::new();
        self.clock.start_now(&mut raised);
        raised.into_iter().map(from_clock).collect()
    }

    /// Adds bonus seconds to the time limit.
    pub fn award_bonus(&mut self, seconds: f32) {
        self.clock.award_bonus(seconds);
    }

    /// Advances the clock by `dt` seconds.
    pub fn tick(&mut self, dt: f32) -> Vec<RaceEvent> {
        let mut raised = Vec::new();
        self.clock.tick(dt, &mut raised);
        let mut events: Vec<RaceEvent> = raised.into_iter().map(from_clock).collect();
        if events.contains(&RaceEvent::TimeUp) {
            self.end(&mut events);
        }
        events
    }

    /// Tells the race how fast racer `index` is going now, m/s: the speed traps read it on the next move.
    pub fn set_speed(&mut self, index: usize, mps: f32) {
        let time = self.clock.elapsed();
        let racing = self.clock.phase() == Phase::Racing;
        let Some(racer) = self.racers.get_mut(index) else { return };
        racer.speed = mps;
        if racing && racer.racing() {
            racer.stats.note_speed(mps, time);
        }
    }

    /// Puts racer `index` at `at` without raising anything (the grid, a reset or a teleport).
    pub fn place_racer(&mut self, index: usize, at: Vec3) {
        if let Some(racer) = self.racers.get_mut(index) {
            racer.position = at;
        }
    }

    /// Moves racer `index` to `to`. During the race this raises the checkpoint, lap and finish events; before it, the
    /// racer is only placed.
    pub fn move_racer(&mut self, index: usize, to: Vec3) -> Vec<RaceEvent> {
        let mut events = Vec::new();
        let phase = self.clock.phase();
        let Some(racer) = self.racers.get_mut(index) else { return events };
        let from = racer.position;
        racer.position = to;
        if phase != Phase::Racing || !racer.racing() {
            return events;
        }
        let mut raised = Vec::new();
        racer.progress.advance(&self.course, from, to, &mut raised);
        racer.percent = self.course.percent(&racer.progress, to);
        let time = self.clock.elapsed();
        racer.stats.note_distance(from.distance(to), time);
        self.note_stage(index, time);
        for event in raised {
            self.apply(index, event, time, &mut events);
        }
        self.pass_traps(index, from, to, &mut events);
        if !events.is_empty() {
            self.check_over(&mut events);
        }
        events
    }

    /// Keeps the time and the place of racer `index` at the stage it has just passed, if any.
    fn note_stage(&mut self, index: usize, time: f32) {
        let Some(stage) = self.racers[index].stats.stage_due(self.racers[index].percent) else { return };
        let place = self.places()[index];
        self.racers[index].stats.stages[stage] = Some(StageMark { time, place });
    }

    /// A move through a speed trap: entering its sphere pays when the speed beats the threshold and the best so far.
    fn pass_traps(&mut self, index: usize, from: Vec3, to: Vec3, out: &mut Vec<RaceEvent>) {
        let speed = self.racers[index].speed;
        for (slot, trap) in self.course.traps.iter().enumerate() {
            if trap.zone.contains(from) || !trap.zone.touches(from, to) {
                continue;
            }
            let kmh = speed / KMH_TO_MS;
            if self.racers[index].traps.record_pass(slot, kmh, trap.threshold_kmh).is_some() {
                out.push(RaceEvent::SpeedTrap { racer: index, slot, kmh });
            }
        }
    }

    fn apply(&mut self, index: usize, event: Event, time: f32, out: &mut Vec<RaceEvent>) {
        match event {
            Event::Checkpoint(number) => {
                self.racers[index].checkpoint_times.push(time);
                out.push(RaceEvent::Checkpoint { racer: index, number });
                let seconds = self.course.bonus(number);
                if self.racers[index].human && seconds > 0.0 {
                    self.clock.award_bonus(seconds);
                    out.push(RaceEvent::Bonus { racer: index, seconds });
                }
            }
            Event::Lap(lap) => {
                self.racers[index].lap_times.push(time);
                out.push(RaceEvent::Lap { racer: index, lap });
                self.knockouts_after(lap, time, out);
            }
            Event::Finished => {
                let racer = &mut self.racers[index];
                racer.lap_times.push(time);
                racer.finished_at = Some(time);
                racer.percent = 100.0;
                out.push(RaceEvent::Finished { racer: index, time });
            }
        }
    }

    /// The knockout rule at the end of `lap`: see [`knockout_due`].
    fn knockouts_after(&mut self, lap: u32, time: f32, out: &mut Vec<RaceEvent>) {
        let completed = self.racers.iter().filter(|r| r.progress.laps >= lap || r.finished_at.is_some()).count();
        if !knockout_due(self.racers.len(), self.config.knockouts_per_lap, lap, completed) {
            return;
        }
        for index in 0..self.racers.len() {
            let racer = &self.racers[index];
            if racer.human || !racer.racing() || racer.progress.laps >= lap {
                continue;
            }
            self.racers[index].out_at = Some(time);
            out.push(RaceEvent::KnockedOut { racer: index });
        }
    }

    /// Takes racer `index` out of the race (knocked out, totalled or busted).
    pub fn knock_out(&mut self, index: usize) -> Vec<RaceEvent> {
        let mut events = Vec::new();
        let time = self.clock.elapsed();
        let Some(racer) = self.racers.get_mut(index) else { return events };
        if !racer.racing() {
            return events;
        }
        racer.out_at = Some(time);
        events.push(RaceEvent::KnockedOut { racer: index });
        self.check_over(&mut events);
        events
    }

    /// Records a speed-trap pass of racer `index` through trap `slot` at `kmh`.
    pub fn speed_trap(&mut self, index: usize, slot: usize, kmh: f32) -> Option<f32> {
        if self.clock.phase() != Phase::Racing {
            return None;
        }
        self.racers.get_mut(index)?.traps.record(slot, kmh)
    }

    /// The race is over when every human racer is done.
    fn check_over(&mut self, out: &mut Vec<RaceEvent>) {
        if self.over {
            return;
        }
        let humans_done = self.racers.iter().filter(|r| r.human).all(Racer::done);
        let any_human = self.racers.iter().any(|r| r.human);
        if any_human && humans_done {
            self.end(out);
        }
    }

    fn end(&mut self, out: &mut Vec<RaceEvent>) {
        if self.over {
            return;
        }
        self.over = true;
        self.clock.finish();
        out.push(RaceEvent::Over);
    }

    /// The ranking inputs of every racer.
    pub fn standings(&self) -> Vec<Standing> {
        let over_time = match self.clock.has_limit() {
            true => (self.clock.elapsed() - self.config.time_limit).max(0.0),
            false => 0.0,
        };
        let loss = overtime_loss(self.config.overtime_penalty, over_time);
        self.racers
            .iter()
            .map(|r| Standing {
                finished_at: r.finished_at,
                out_at: r.out_at,
                laps: r.progress.laps,
                percent: r.percent,
                points: (r.traps.total() - loss).max(0.0),
            })
            .collect()
    }

    /// The place of every racer now, 1 for first.
    pub fn places(&self) -> Vec<u32> {
        places(&self.standings(), self.config.rank_by_points)
    }

    /// How every racer ended (or stands now, if the race is still on).
    pub fn outcomes(&self) -> Vec<Outcome> {
        let places = self.places();
        self.racers
            .iter()
            .enumerate()
            .map(|(racer, r)| Outcome {
                racer,
                place: places[racer],
                time: r.finished_at,
                finished: r.finished_at.is_some(),
            })
            .collect()
    }
}

fn from_clock(event: ClockEvent) -> RaceEvent {
    match event {
        ClockEvent::Count(n) => RaceEvent::Count(n),
        ClockEvent::Go => RaceEvent::Go,
        ClockEvent::TimeUp => RaceEvent::TimeUp,
    }
}
