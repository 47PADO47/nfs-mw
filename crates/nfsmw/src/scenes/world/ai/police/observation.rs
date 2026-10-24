//! Physical target observations and the pursuit's fractional-rate decision clock.
//! Spec: `police-ai-parity-core.md`, `ai-pursuit-cop-cars.md` section 1.

use glam::Vec3;

use super::{PlayerView, Police};
use crate::scenes::world::StateHasher;

/// The original task rate is one decision per four updates, not a quarter second.
pub(super) const DECISION_PERIOD: f32 = 4.0 * crate::scenes::world::drive::STEP;

#[derive(Debug)]
pub(super) struct Observation {
    bank: f64,
    position: Option<Vec3>,
    distance: f32,
    speed: f32,
    jerk: bool,
    nitrous: bool,
    nitrous_input: bool,
    nitrous_boost: f32,
    pub(super) ground_sight_age: Option<f32>,
    pub(super) secondary_seen: bool,
    support_busy: bool,
    speech_phase: Option<blackbox_pursuit::siren::Phase>,
}

impl Default for Observation {
    fn default() -> Self {
        Self {
            bank: 0.0,
            position: None,
            distance: 1000.0,
            speed: 0.0,
            jerk: false,
            nitrous: false,
            nitrous_input: false,
            nitrous_boost: 0.0,
            ground_sight_age: None,
            secondary_seen: false,
            support_busy: false,
            speech_phase: None,
        }
    }
}

impl Observation {
    pub(super) fn collect(&mut self, dt: f32, player: &PlayerView) {
        self.bank += f64::from(dt);
        self.nitrous = player.nitrous > 0.0;
        self.nitrous_input = player.nos_input;
        self.nitrous_boost = player.nitrous_boost;
    }

    pub(super) fn take_decision(&mut self) -> bool {
        let period = f64::from(DECISION_PERIOD);
        if self.bank + 1e-6 < period {
            return false;
        }
        self.bank = (self.bank - period).max(0.0);
        true
    }

    pub(super) fn observe(&mut self, player: &PlayerView) {
        let previous = self.position.unwrap_or(player.position);
        let blend = 0.1 * DECISION_PERIOD;
        let lag = previous.lerp(player.position, blend);
        self.position = Some(lag);
        self.distance += (player.position.distance(lag) - self.distance) * blend;
        self.speed += (player.velocity.length() - self.speed) * blend * 0.5;
        let factor = match self.distance > 0.01 {
            true => self.speed * 10.0 / self.distance,
            false => 0.0,
        };
        if factor >= 3.0 {
            self.jerk = true;
        }
        if factor <= 1.75 {
            self.jerk = false;
        }
    }

    pub(super) fn hash_into(&self, hash: &mut StateHasher) {
        hash.u64(self.bank.to_bits());
        hash.bool(self.position.is_some());
        if let Some(position) = self.position {
            hash.vec3(position);
        }
        hash.f32(self.distance);
        hash.f32(self.speed);
        hash.bool(self.jerk);
        hash.bool(self.nitrous);
        hash.bool(self.nitrous_input);
        hash.f32(self.nitrous_boost);
        hash.option_f32(self.ground_sight_age);
        hash.bool(self.secondary_seen);
        hash.bool(self.support_busy);
        hash.bytes(format!("{:?}", self.speech_phase).as_bytes());
    }
}

impl Police {
    pub(in crate::scenes::world::ai) fn set_ground_sight_age(&mut self, age: f32) {
        self.observation.ground_sight_age = Some(age);
    }

    #[cfg(test)]
    pub(in crate::scenes::world::ai) fn observed_ground_age(&self) -> Option<f32> {
        self.observation.ground_sight_age
    }

    pub(in crate::scenes::world::ai) fn set_secondary_visibility(&mut self, seen: bool) {
        self.observation.secondary_seen = seen;
    }
    pub(in crate::scenes::world::ai) fn speech_phase(&self) -> Option<blackbox_pursuit::siren::Phase> {
        self.observation.speech_phase
    }
    pub(in crate::scenes::world::ai) fn set_support_busy(&mut self, busy: bool) {
        self.observation.support_busy = busy;
    }

    pub(super) fn support_busy(&self) -> bool {
        self.observation.support_busy
    }

    pub fn is_jerk(&self) -> bool {
        self.observation.jerk
    }

    pub fn target_nitrous(&self) -> bool {
        self.observation.nitrous
    }

    pub fn target_nitrous_input(&self) -> bool {
        self.observation.nitrous_input
    }
    pub fn target_nitrous_boost(&self) -> f32 {
        self.observation.nitrous_boost
    }

    pub(in crate::scenes::world::ai) fn hash_observation(&self, hash: &mut StateHasher) {
        self.observation.hash_into(hash);
    }
}

impl super::super::TrafficWorld {
    pub fn note_police_speech(&mut self, phase: Option<blackbox_pursuit::siren::Phase>) {
        use blackbox_pursuit::siren::Phase;
        self.police.set_speech_enabled(phase.is_some());
        self.police.set_speech_setup(phase == Some(Phase::Opening));
        self.police.observation.speech_phase = phase;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(at: Vec3, velocity: Vec3) -> PlayerView {
        PlayerView {
            nos_input: false,
            nitrous_boost: 0.0,
            position: at,
            velocity,
            forward: Vec3::X,
            speed: velocity.length(),
            nitrous: 0.0,
            throttle: 0.0,
        }
    }

    #[test]
    fn a_task_rate_is_four_physics_ticks_and_retains_all_elapsed_time() {
        let mut observation = Observation::default();
        let view = view(Vec3::ZERO, Vec3::ZERO);
        for i in 1..=240 {
            observation.collect(crate::scenes::world::drive::STEP, &view);
            assert_eq!(observation.take_decision(), i % 4 == 0);
        }
        observation.collect(DECISION_PERIOD * 3.0, &view);
        assert!(observation.take_decision());
        assert!(observation.take_decision());
        assert!(observation.take_decision());
        assert!(!observation.take_decision());
        observation.collect(2.0, &view);
        let mut decisions = 0;
        while observation.take_decision() {
            decisions += 1;
        }
        assert_eq!(decisions, 30);
    }

    #[test]
    fn circling_triggers_the_jerk_boost_and_straight_travel_clears_it() {
        let mut observation = Observation::default();
        for i in 0..3000 {
            let a = i as f32 * DECISION_PERIOD;
            observation.observe(&view(Vec3::new(a.cos(), a.sin(), 0.0) * 20.0, Vec3::X * 30.0));
        }
        assert!(observation.jerk);
        for i in 0..3000 {
            observation.observe(&view(Vec3::X * i as f32 * DECISION_PERIOD * 30.0, Vec3::X * 30.0));
        }
        assert!(!observation.jerk);
    }

    #[test]
    fn empty_tank_input_remains_distinct_from_nitrous_engagement() {
        let mut observation = Observation::default();
        let player = PlayerView { nos_input: true, ..view(Vec3::ZERO, Vec3::ZERO) };
        observation.collect(crate::scenes::world::drive::STEP, &player);
        assert!(observation.nitrous_input);
        assert!(!observation.nitrous);
    }
}
