//! Free-roam radar and the pursuit counters; authored objects, no placeholder text.

use blackbox_feng::{ObjectRef, PackageId, Runtime};

use super::HudState;

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
    heat_labels: Vec<ObjectRef>,
    heat_group: Option<ObjectRef>,
    engaged: Option<ObjectRef>,
    damaged: Option<ObjectRef>,
    radar: super::radar::Binding,
}

impl Binding {
    pub fn new(rt: &Runtime, package: PackageId) -> Self {
        let heat_labels = rt
            .package(package)
            .into_iter()
            .flat_map(|p| p.objects.iter().enumerate())
            .filter(|(_, object)| {
                object.kind == blackbox_feng::package::ObjectKind::String
                    && matches!(object.name_hash, 0xA81A_2028 | 0x7F91_DA62)
            })
            .map(|(index, _)| ObjectRef { package, index })
            .collect();
        Self {
            heat_labels,
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
        for &label in &self.heat_labels {
            rt.set_text(label, format!("x{}", state.vehicle_heat.floor().clamp(1.0, 10.0) as u32));
            rt.set_alpha(label, 255);
        }
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
