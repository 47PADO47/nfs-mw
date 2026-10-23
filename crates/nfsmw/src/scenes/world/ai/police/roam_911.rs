//! The world's side of the free-roam 911 call: what the player smashes and the traffic it hits are counted while no cop
//! chases ([`cop_manager::roam`](super::super::cop_manager::roam)), told to the speech machine as a free-roam frame,
//! and the call the machine makes starts the player's 911 timer and clears the cop lockout. Specs:
//! `docs/specs/ai-pursuit-heat.md` section 5.3 and `docs/specs/ai-pursuit-speech.md` section 8.

use glam::Vec3;
use nfsmw_data::speech_chase::{chase_tune, level_speech};

use super::super::cop_manager::roam::{Event, Level, Tuning};
use super::*;
use crate::audio::{PlayerFeed, RoamEvent, RoamFeed};

impl super::super::TrafficWorld {
    /// Reads the thresholds of the install: how hard a traffic hit is and the 911 figures of every heat row.
    pub fn load_911_data(&mut self, db: &blackbox_attrib::Database) {
        let tuning = Tuning::of(&chase_tune(db));
        let levels = (1..=10).map(|heat| Level::of(&level_speech(db, heat))).collect();
        self.cops.roam.set_data(tuning, levels);
    }

    /// Runs the 911 clocks. A cop that chases the player, or the arrest in progress, clears the counters.
    pub(in crate::scenes::world::ai) fn update_911(&mut self, dt: f32) {
        let chasing = self.is_pursuing() || self.police.bust.active();
        let heat = self.police.patrol_heat();
        if let Some(lifetime) = self.cops.roam.tick(dt, chasing, heat) {
            log::info!("911: the speech has not answered, so the call is made without it");
            self.start_911_call(lifetime);
        }
    }

    /// The dispatcher's 911 report was said (or the fallback ran out): the player's 911 timer runs `secs` seconds and the
    /// cops are let in (`Force911State`: `Set911CallTime` and `LockoutCops(false)`). A patrol cop that sees the player
    /// now chases it whatever its speed.
    pub fn start_911_call(&mut self, secs: f32) {
        if !self.cops.roam.start_call(secs) {
            return;
        }
        self.cops.unlock();
        log::info!("911: a call was made, the cops are out for {secs:.0} s");
    }

    /// The props the player hit this step, with their cost to state.
    pub(in crate::scenes::world::ai) fn note_smacked(&mut self, hits: &[(u32, u32)]) {
        for &(id, cost) in hits {
            self.cops.roam.note_prop(id, cost);
        }
    }

    /// The traffic cars (by car index) the player hit this step with the closing speed (m/s).
    pub(in crate::scenes::world::ai) fn note_traffic_hits(&mut self, hits: &[(usize, f32)]) {
        for &(owner, closing) in hits {
            let Some(car) = self.cars.get(owner) else { continue };
            self.cops.roam.note_traffic(car.key(), closing);
        }
    }

    /// What the speech machine reads while no cop chases: the player, the heat, the lockout and what the player hit
    /// since the last call. `None` with a chase on (its own feed carries the speech) or before the player is seen.
    pub fn roam_feed(&mut self, tuning: &Tuning) -> Option<RoamFeed> {
        if self.is_pursuing() {
            return None;
        }
        let view = self.speech_view?;
        let at = Vec3::from_array(space::to_physics(view.position));
        let forward = Vec3::from_array(space::to_physics(view.forward)).normalize_or_zero();
        let player = PlayerFeed {
            position: at.to_array(),
            velocity_dir: match view.speed.abs() > 0.5 {
                true => (forward * view.speed.signum()).to_array(),
                false => [0.0; 3],
            },
            forward: forward.to_array(),
            road: self.road_speech_id(at, forward),
            car_type: 0,
            car_colour: 0,
            ..Default::default()
        };
        let events = self
            .cops
            .roam
            .take_events()
            .into_iter()
            .map(|event| match event {
                Event::Smashed { cost } => RoamEvent::Smashed { cost },
                Event::HitTraffic { closing } => RoamEvent::HitTraffic { intensity: tuning.intensity(closing) },
            })
            .collect();
        Some(RoamFeed {
            player,
            speed_mph: view.speed.abs() / super::super::traffic::MPH,
            heat: self.police.patrol_heat(),
            lockout_remaining: self.cops.lockout_left(),
            events,
            restarted: self.cops.roam.take_restarted(),
            armed: self.cops.roam.armed(),
        })
    }

    /// The traffic-hit judgement the feed's events use.
    pub fn roam_tuning(&self) -> Tuning {
        self.cops.roam.tuning()
    }

    /// The `pursuit 911` command: makes the call now with the heat row's lifetime (a development shortcut).
    pub fn dev_911_call(&mut self) -> String {
        let secs = self.cops.roam.row().lifetime;
        self.cops.roam.restart();
        self.start_911_call(secs);
        format!("a 911 call was made: {}", self.call_911_status())
    }

    /// One line for the console: the call, the counters against the heat row's figures.
    pub fn call_911_status(&self) -> String {
        self.cops.roam.status()
    }
}
