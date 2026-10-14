//! Audio-facing physical contacts; spec: `car-audio-integration.md`.

use glam::Vec3;
use nfsmw_data::sound::EventKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContactSource {
    World,
    Object,
    Ground,
}

/// Captured before response, with severity in the response's effective delta-v units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarContact {
    pub source: ContactSource,
    pub identity: u64,
    pub surface: u32,
    pub point: Vec3,
    pub normal: Vec3,
    pub velocity: Vec3,
    pub delta_v: f32,
    pub front: bool,
    pub bottom: bool,
}

impl CarContact {
    pub fn render_point(self) -> Vec3 {
        Vec3::new(self.point.z, -self.point.x, self.point.y)
    }
    pub fn hit_kind(self) -> EventKind {
        match (self.source, self.bottom) {
            (ContactSource::Ground, true) => EventKind::BottomOut,
            (ContactSource::Ground, false) => EventKind::HitGround,
            (ContactSource::World, _) => EventKind::HitWorld,
            (ContactSource::Object, _) => EventKind::HitObject,
        }
    }

    pub fn scrape_kind(self) -> EventKind {
        match (self.source, self.bottom) {
            (ContactSource::Ground, true) => EventKind::BottomScrape,
            (ContactSource::Ground, false) => EventKind::ScrapeGround,
            (ContactSource::World, _) => EventKind::ScrapeWorld,
            (ContactSource::Object, _) => EventKind::ScrapeObject,
        }
    }

    pub fn closing_speed(self) -> f32 {
        (-self.velocity.dot(self.normal)).max(0.0)
    }

    pub fn sliding_speed(self) -> f32 {
        (self.velocity - self.normal * self.velocity.dot(self.normal)).length()
    }
}
