//! Ordered traffic producers and their authored modules. Spec: authored-traffic-audio.md.
mod controller;
mod emitter;
mod mixer;
mod selection;
#[cfg(test)]
mod tests;

use super::{Audio, AudioView};
pub(super) use controller::TrafficAudio;
use glam::Vec3;

pub const HORN_BANK: &str = "IG_GLOBAL/ENV_COMMON_MB.abk";
pub const WHOOSH_BANK: &str = "IG_GLOBAL/Stitch_Whoosh_MB.abk";
const ENGINE_BANK: &str = "IG_GLOBAL/TRAFFIC_MB.abk";

/// Installed IDs belong to class data, never directly to a bank sample index.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TrafficProfile {
    pub horn: Option<u32>,
    pub engine: Option<u32>,
    pub whoosh: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrafficEmitter {
    pub id: u32,
    pub profile: TrafficProfile,
    pub position: Vec3,
    pub forward: Vec3,
    pub velocity: Vec3,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrafficFrame {
    pub dt: f32,
    pub listener_position: Vec3,
    pub listener_forward: Vec3,
    pub listener_velocity: Vec3,
    pub emitters: Vec<TrafficEmitter>,
}

/// Ordered events followed by a complete snapshot of the same simulation step.
#[derive(Debug, Clone, PartialEq)]
pub enum TrafficSound {
    Horn { emitter: u32, duration: f32 },
    NearMiss { emitter: u32, relative_speed: f32 },
    Frame(TrafficFrame),
}

impl Audio {
    pub fn update_traffic(&mut self, sounds: Vec<TrafficSound>, mut view: AudioView) {
        view.speedbreaker_mix = self.speedbreaker.snapshot();
        let mut traffic = std::mem::take(&mut self.traffic);
        traffic.update(self, sounds, view);
        self.traffic = traffic;
    }

    pub fn stop_traffic(&mut self) {
        self.traffic = TrafficAudio::default();
    }
}
