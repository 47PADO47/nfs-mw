//! The sirens of the police cars: one state machine per car (`blackbox_pursuit::siren`), run from the police state and
//! handed to the audio as a frame of sources. Spec: `docs/specs/cop-siren-audio.md` section 2.

use std::collections::HashMap;

use blackbox_pursuit::formation::collapse;
use blackbox_pursuit::siren::{Input, Machine, OpeningGap, Phase, State, Tuning};
use glam::Vec3;
use nfsmw_data::pursuit_siren::SirenData;

use super::TrafficWorld;
use crate::audio::{SirenFrame, SirenSource};

/// The opening exchange of a chase lasts this long, seconds (the speech focus sits on the pursuit flow for the first
/// 5 s of the chase). [decomp]
const OPENING_SECS: f32 = 5.0;
/// Contact counts as lost after this long without a sighting: the speech's no-contact timer (2 s) plus the time
/// considered lost without line of sight (3 s). [decomp]
const LOST_SECS: f32 = 5.0;

/// What the chase tells every car.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Chase {
    /// A chase is on.
    pub on: bool,
    pub heat: u32,
    /// Seconds since the chase began.
    pub secs: f32,
    /// Seconds since any cop saw the player.
    pub unseen: f32,
    /// A formation is closing in or a finisher runs.
    pub pressing: bool,
}

/// What the world knows of one police car.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct CarFacts {
    pub id: u32,
    pub position: Vec3,
    pub velocity: Vec3,
    /// A cop of the chase.
    pub cop: bool,
    pub in_formation: bool,
    pub roadblock: bool,
    pub lights_on: bool,
    pub wrecked: bool,
}

/// Where the chase is as far as the siren cares: the opening exchange, lost contact, or the chase proper.
pub(super) fn phase(chase: &Chase) -> Phase {
    if chase.secs < OPENING_SECS {
        return Phase::Opening;
    }
    if chase.unseen >= LOST_SECS {
        return Phase::Lost;
    }
    Phase::Running
}

/// The machine's input for one car.
pub(super) fn input(chase: &Chase, intense: bool, car: &CarFacts) -> Input {
    Input {
        in_pursuit: chase.on && car.cop,
        phase: phase(chase),
        high_intensity: intense,
        pressing: chase.pressing,
        in_formation: car.in_formation,
        in_roadblock: car.roadblock,
        lights_on: car.lights_on,
        destroyed: car.wrecked,
    }
}

/// The sirens of the world's police cars.
#[derive(Default)]
pub(super) struct Sirens {
    data: SirenData,
    clock: f64,
    gap: OpeningGap,
    machines: HashMap<u32, Machine>,
    random: u32,
    /// The states of the last frame, by car, for the status line.
    states: Vec<(u32, State)>,
}

impl Sirens {
    /// Gives the sirens the install's timings.
    pub fn set_data(&mut self, data: SirenData) {
        self.data = data;
    }

    fn next_random(&mut self) -> f32 {
        // xorshift32: the opening gap only needs to differ from chase to chase, and the world's own generator must
        // not be disturbed.
        let mut x = if self.random == 0 { 0x2545_F491 } else { self.random };
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.random = x;
        (x >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Advances every car's machine by `dt` seconds. Cars that are gone lose theirs.
    pub fn tick(&mut self, dt: f32, chase: &Chase, cars: &[CarFacts]) -> Vec<SirenSource> {
        self.clock += f64::from(dt);
        self.machines.retain(|id, _| cars.iter().any(|car| car.id == *id));
        let row = self.data.row(chase.heat.max(1));
        let tuning = Tuning {
            init_min_period: row.init_min_period,
            init_variation: row.init_variation,
            wail_period: row.wail_period,
            max_yelp_time: row.max_yelp_time,
            scream_period: row.scream_period,
            max_scream_time: row.max_scream_time,
        };
        let intense = self.data.is_high_intensity(chase.heat, chase.secs);
        let now = self.clock;
        let mut sources = Vec::with_capacity(cars.len());
        for car in cars {
            let random = self.next_random();
            let machine = self.machines.entry(car.id).or_insert_with(|| Machine::new(now));
            let state = machine.update(now, &input(chase, intense, car), &tuning, &mut self.gap, random);
            sources.push(SirenSource { id: car.id, state, position: car.position, velocity: car.velocity });
        }
        self.states = sources.iter().map(|s| (s.id, s.state)).collect();
        sources
    }

    /// Forgets every car (no police left).
    pub fn clear(&mut self) {
        self.machines.clear();
        self.states.clear();
    }

    /// The states of the last frame.
    pub fn states(&self) -> &[(u32, State)] {
        &self.states
    }
}

impl TrafficWorld {
    /// The police cars' sirens for the audio, one source per police car; `None` with no police on the road.
    pub fn siren_frame(&mut self, dt: f32) -> Option<SirenFrame> {
        if !dt.is_finite() || dt <= 0.0 {
            return None;
        }
        let cars: Vec<_> = self
            .cars
            .iter()
            .filter(|car| car.is_police())
            .map(|car| {
                let emitter = car.audio_emitter();
                CarFacts {
                    id: emitter.id,
                    position: emitter.position,
                    velocity: emitter.velocity,
                    cop: car.is_cop(),
                    in_formation: car.is_cop() && !car.is_roadblock() && car.support_serial().is_none(),
                    roadblock: car.is_roadblock(),
                    lights_on: car.emergency_lights(),
                    wrecked: car.is_wrecked(),
                }
            })
            .collect();
        if cars.is_empty() {
            self.sirens.clear();
            return None;
        }
        let level = self.police.level();
        let formation = self.police.last_formation().is_some();
        let slow = level.is_some_and(|level| self.player_speed * 3.6 < collapse::level(level).speed_kmh);
        let chase = Chase {
            on: level.is_some() && self.is_pursuing(),
            heat: level.unwrap_or_else(|| self.police.patrol_heat()),
            secs: self.pursuit_time(),
            unseen: self.police.seconds_unseen(),
            pressing: self.police.finisher_running() || (formation && slow),
        };
        // A pressing formation only involves the cars that hold a formation slot.
        let cars: Vec<_> =
            cars.into_iter().map(|car| CarFacts { in_formation: car.in_formation && formation, ..car }).collect();
        let sources = self.sirens.tick(dt, &chase, &cars);
        Some(SirenFrame { dt, sources })
    }

    /// The siren state of every police car as of the last audio frame, for the `traffic sirens` command. The audio
    /// plays at most four of them (`docs/specs/cop-siren-audio.md` section 3).
    pub fn siren_status(&self) -> String {
        let states = self.sirens.states();
        if states.is_empty() {
            return "no police cars".into();
        }
        let cars: Vec<String> = states.iter().map(|(id, state)| format!("#{id} {state:?}")).collect();
        format!("{} police cars: {}", states.len(), cars.join(", "))
    }
}

#[cfg(test)]
mod tests;
