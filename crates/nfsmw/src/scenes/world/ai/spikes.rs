//! Spike strips on the road: the player's tire that runs over a strip is punctured, and the tire model's blow-out
//! follows (`blackbox_vehicle` tires). The geometry and the test are in `blackbox_pursuit::spikes`. Spec:
//! `docs/specs/ai-pursuit-roadblocks.md` section 6. Only the player's car is spiked here: the racers' spiking is not built.

use blackbox_pursuit::spikes;
use glam::Vec3;

use super::TrafficWorld;
use crate::scenes::world::drive::CarSim;

impl TrafficWorld {
    /// Procedural strip geometry on the exact footprint used for tire contact, for a world whose installed spike-strip
    /// model is not loaded (the model is drawn by `roadblock_instances`).
    pub fn spike_geometry(&self, out: &mut Vec<blackbox_gfx::EffectVertex>) {
        if self.roadblocks.has_models() {
            return;
        }
        for strip in &self.roadblocks.strips() {
            let facing = glam::Vec2::new(strip.heading.cos(), strip.heading.sin());
            let across = glam::Vec2::new(-facing.y, facing.x);
            let point = |along: f32, side: f32, height: f32| {
                let p = strip.position + facing * along + across * side;
                crate::scenes::world::space::to_render([p.x, strip.elevation + height, p.y])
            };
            let a = strip.half[0];
            let b = strip.half[1];
            blackbox_gfx::EffectLayer::quad(
                out,
                [point(-a, -b, 0.04), point(a, -b, 0.04), point(a, b, 0.04), point(-a, b, 0.04)],
                [35, 35, 35, 255],
            );
            for tooth in 0..16 {
                let side = -b + (tooth as f32 + 0.5) * 2.0 * b / 16.0;
                let base = point(0.0, side, 0.05);
                let tip = base + Vec3::Z * 0.15;
                blackbox_gfx::EffectLayer::quad(
                    out,
                    [point(-a * 0.5, side - 0.05, 0.05), point(a * 0.5, side + 0.05, 0.05), tip, tip],
                    [195, 195, 180, 255],
                );
            }
        }
    }

    /// Where the strips lie, in render space.
    pub fn spike_sites(&self) -> Vec<[f32; 2]> {
        self.roadblocks
            .strips()
            .iter()
            .map(|strip| {
                let at = crate::scenes::world::space::to_render([strip.position.x, strip.elevation, strip.position.y]);
                [at.x, at.y]
            })
            .collect()
    }

    /// Tires of the player punctured by strips since the world opened.
    pub fn spike_hits(&self) -> u32 {
        self.spike_hits
    }

    /// Punctures each grounded tire of the player that is over a strip. A tire already punctured is left alone.
    pub(super) fn spike_player(&mut self, player: &mut CarSim) {
        let wheels = player.wheel_patches();
        let previous = self.spike_patches.replace(wheels);
        let strips = self.roadblocks.strips();
        if strips.is_empty() {
            return;
        }
        let punctured = match previous.filter(|old| old[0].position.distance(wheels[0].position) < 50.0) {
            Some(previous) => spikes::punctured_sweep(&strips, &previous, &wheels),
            None => spikes::punctured(&strips, &wheels),
        };
        for (wheel, hit) in punctured.into_iter().enumerate() {
            if hit && player.puncture_wheel(wheel) {
                self.spike_hits += 1;
                log::info!("spike strip: tire {wheel} punctured");
                self.note_speech(crate::audio::WorldEvent::TireBlown);
            }
        }
    }

    /// Takes the roadblocks, and so their spike strips, away: the chase was ended by hand or by the arrest.
    pub(super) fn clear_spikes(&mut self) {
        self.roadblocks.clear();
        self.spike_patches = None;
    }
}
