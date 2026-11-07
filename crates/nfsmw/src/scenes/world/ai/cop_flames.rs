//! The nitrous flames of the police cars. Every car's tail-pipe effects play its nitrous group while its nitrous burns
//! (`docs/specs/exhaust-flames.md` section 2); a cop has no shift timing, so it shows no gear-change flame and only the
//! nitrous one. The flames are the player's [`ExhaustFlames`], one per cop car that has burned nitrous near the camera,
//! loaded on the frame after its first burn and freed when the car is gone, far away or the setting is off.

use blackbox_attrib::Database;
use blackbox_gfx::RenderBackend;
use blackbox_render::EffectLayer;
use game_install::GameDir;
use glam::Vec3;

use super::TrafficWorld;
use crate::scenes::world::exhaust::ExhaustFlames;

/// Cop cars farther than this from the camera (render space, metres) have no flames: the original skips the pipe effects
/// of cars beyond a distance factor whose value is not mapped **[ours]**.
const DISTANCE: f32 = 200.0;
/// Flames loaded on one frame: each load reads the particle textures from the install.
const LOADS_PER_FRAME: usize = 1;

/// The flames of the police cars, by the car's render key.
#[derive(Default)]
pub(in crate::scenes::world) struct CopFlames {
    cars: Vec<(u32, ExhaustFlames)>,
}

impl TrafficWorld {
    /// Once per frame, with the renderer: frees the flames of cars that are gone, not cops, far from `camera` or of a
    /// setting turned off, and loads the flames of near cops whose nitrous burns.
    pub(in crate::scenes::world) fn maintain_cop_flames(
        &mut self,
        renderer: &mut dyn RenderBackend,
        dir: &GameDir,
        db: &Database,
        enabled: bool,
        camera: Vec3,
    ) {
        let near = |key: u32| {
            self.cars
                .iter()
                .find(|car| car.render_key() == key)
                .filter(|car| enabled && car.is_cop() && car.position().distance(camera) < DISTANCE)
        };
        let (kept, gone): (Vec<_>, Vec<_>) =
            std::mem::take(&mut self.cop_flames.cars).into_iter().partition(|(key, _)| near(*key).is_some());
        for (_, mut flames) in gone {
            flames.unload();
            flames.release(renderer);
        }
        self.cop_flames.cars = kept;
        if !enabled {
            return;
        }
        let mut loads = 0;
        for car in &self.cars {
            if loads == LOADS_PER_FRAME {
                return;
            }
            let key = car.render_key();
            let wanted = car.is_cop() && car.nitrous_burning() && car.position().distance(camera) < DISTANCE;
            if !wanted || self.cop_flames.cars.iter().any(|(k, _)| *k == key) {
                continue;
            }
            let mut flames = ExhaustFlames::default();
            flames.set_enabled(true);
            flames.load(renderer, dir, db, car.model());
            self.cop_flames.cars.push((key, flames));
            loads += 1;
        }
    }

    /// One physics step of every cop car's flames, from the car's pose, speed, gear, throttle and nitrous.
    pub(super) fn step_cop_flames(&mut self, dt: f32) {
        for (key, flames) in &mut self.cop_flames.cars {
            match self.cars.iter().find(|car| car.render_key() == *key) {
                Some(car) => flames.step(dt, &car.flame_state()),
                None => flames.age(dt),
            }
        }
    }

    /// Appends the cop cars' flame batches to `layer`; [`Self::reclaim_cop_flames`] takes the buffers back after the
    /// upload.
    pub(in crate::scenes::world) fn cop_flame_geometry(
        &mut self,
        camera: Vec3,
        forward: Vec3,
        world: &blackbox_collision::CollisionWorld,
        layer: &mut EffectLayer,
    ) {
        for (_, flames) in &mut self.cop_flames.cars {
            flames.geometry_in_world(camera, forward, world, layer);
        }
    }

    pub(in crate::scenes::world) fn reclaim_cop_flames(&mut self, layer: &mut EffectLayer) {
        for (_, flames) in &mut self.cop_flames.cars {
            flames.reclaim(layer);
        }
    }

    /// Frees every cop car's flames (the scene closes).
    pub(super) fn release_cop_flames(&mut self, renderer: &mut dyn RenderBackend) {
        for (_, mut flames) in self.cop_flames.cars.drain(..) {
            flames.unload();
            flames.release(renderer);
        }
    }
}
