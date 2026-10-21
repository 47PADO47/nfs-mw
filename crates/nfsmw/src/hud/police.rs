//! Free-roam radar and the pursuit counters; authored objects, no placeholder text.

use blackbox_feng::{ObjectRef, PackageId, Runtime};

use super::HudState;

mod heat;

/// Original PC ground detector range, metres (`docs/specs/hud-pursuit-state.md`).
pub const RADAR_RANGE: f32 = 300.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RadarHud {
    /// Signed angle from the player's heading toward the cop (left positive).
    pub angle: f32,
    pub distance: f32,
}

impl RadarHud {
    pub fn normalized_range(self) -> f32 {
        (self.distance / RADAR_RANGE).clamp(0.0, 1.0)
    }
}

pub(super) struct Binding {
    heat: heat::Binding,
    heat_group: Option<ObjectRef>,
    engaged: Option<ObjectRef>,
    damaged: Option<ObjectRef>,
    radar: super::radar::Binding,
}

impl Binding {
    pub fn new(rt: &Runtime, package: PackageId) -> Self {
        Self {
            heat: heat::Binding::new(rt, package),
            heat_group: rt.find(package, 0xC46A_80A9),
            engaged: rt.find(package, 0x8149_18CA),
            damaged: rt.find(package, 0x5FA7_0D4C),
            radar: super::radar::Binding::new(rt, package),
        }
    }

    pub fn advance(&mut self, dt: f32) {
        self.radar.advance(dt);
    }

    pub fn apply(&mut self, rt: &mut Runtime, state: &HudState, mirror: bool) {
        self.radar.apply(rt, state, mirror);
        self.heat.apply(rt, state.vehicle_heat);
        if let Some(group) = self.heat_group {
            rt.set_alpha(group, 255);
        }
        for (object, count) in [(self.engaged, state.cops_engaged), (self.damaged, state.cops_damaged)] {
            if let Some(object) = object {
                rt.set_text(object, count.to_string());
                rt.set_alpha(object, 255);
            }
        }
    }
}
