//! Repaint the player without resetting physics, damage or camera state.

use blackbox_gfx::RenderBackend;
use glam::Vec3;
use nfsmw_data::car::CarModel;

use super::{CarRig, Drive};

impl Drive {
    #[cfg(test)]
    pub(in crate::scenes::world) fn test_model(&self) -> &CarModel {
        self.rig.model()
    }

    pub(in crate::scenes::world) fn set_tuning(
        &mut self,
        selection: nfsmw_data::car::physics::tuning::TuningSelection,
        tunings: blackbox_vehicle::Tunings,
    ) {
        self.tuning = selection;
        self.tunings = tunings;
        if let Some(sim) = self.sim.as_mut() {
            sim.set_tunings(tunings);
        }
    }

    pub(in crate::scenes::world) fn set_performance(
        &mut self,
        physics: nfsmw_data::car::physics::CarPhysics,
        package: nfsmw_data::car::physics::PerformancePackage,
    ) {
        if let Some(sim) = self.sim.as_mut() {
            sim.set_performance(&physics);
        }
        self.physics = physics;
        self.performance = package;
        self.flames.set_performance(package);
        if let Some(sim) = &self.sim {
            self.telemetry = sim.telemetry();
        }
    }

    pub(in crate::scenes::world) fn recolor(
        &mut self,
        renderer: &mut dyn RenderBackend,
        rgb: [u8; 3],
        sun: Vec3,
    ) -> anyhow::Result<()> {
        self.rig.recolor(renderer, rgb, sun)
    }
    pub(in crate::scenes::world) fn replace_model(
        &mut self,
        renderer: &mut dyn RenderBackend,
        model: CarModel,
        shading: crate::settings::CarShading,
        sun: Vec3,
    ) {
        let rig = CarRig::upload(renderer, crate::scenes::world::keys::PLAYER_CAR, model, shading, sun);
        if let Some(sim) = self.sim.as_mut() {
            sim.set_rest_heights(rig.rest_heights());
            sim.set_visual_tires(rig.visual_tires());
        }
        std::mem::replace(&mut self.rig, rig).release(renderer);
        // The flames were read from the old model's pipes; the next frame reads the new ones.
        self.flames.unload();
        self.tail_trails.clear();
        self.set_damaged_windows(renderer, sun);
    }
}
