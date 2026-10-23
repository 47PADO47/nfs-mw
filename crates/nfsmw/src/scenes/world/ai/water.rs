//! Nearby actor tire water shares the player's weather and authored surface groups.
use super::TrafficWorld;
use crate::audio::environment_context::EnvironmentMap;
use crate::scenes::world::{space, vehicle_effects::TireWaterEffects};
use blackbox_attrib::Database;
use blackbox_collision::CollisionWorld;
use glam::Vec3;
use nfsmw_data::vehicle_effects::TireWaterData;
use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct Water {
    data: Option<TireWaterData>,
    actors: BTreeMap<u32, TireWaterEffects>,
}
pub(in crate::scenes::world) struct RainView<'a> {
    pub camera: Vec3,
    pub intensity: f32,
    pub environment: Option<&'a EnvironmentMap>,
    pub dt: f32,
}
impl TrafficWorld {
    pub(in crate::scenes::world) fn step_rain(
        &mut self,
        db: &Database,
        collision: &CollisionWorld,
        view: RainView<'_>,
    ) {
        if !view.dt.is_finite() || view.dt <= 0.0 {
            return;
        }
        if self.water.data.is_none() && view.intensity > 0.0 {
            self.water.data = Some(TireWaterData::read(db));
        }
        let Some(data) = &self.water.data else {
            return;
        };
        self.water.actors.retain(|id, _| self.cars.iter().any(|car| car.key() == *id));
        for car in &self.cars {
            let near = car.position().distance_squared(view.camera) < 3600.0;
            let key = car.key();
            if !near && !self.water.actors.contains_key(&key) {
                continue;
            }
            let group = self.water.actors.entry(key).or_insert_with(|| TireWaterEffects::new(data.clone()));
            let environment = view
                .environment
                .map(|map| map.at(Vec3::from_array(space::to_physics(car.position()))))
                .unwrap_or_default();
            group.active = near
                && view.intensity > 0.0
                && !(environment.tunnel || environment.overpass || environment.garage || environment.dynamic);
            group.age(view.dt);
            if !group.active {
                group.disconnect();
                continue;
            }
            let (contacts, pose, velocity) = car.rain_contacts(collision);
            group.step(contacts, pose, velocity, view.dt);
        }
    }
    pub(in crate::scenes::world) fn rain_particles(
        &self,
        forward: Vec3,
        textures: &std::collections::HashMap<u32, (blackbox_gfx::TextureHandle, blackbox_gfx::BlendMode)>,
        out: &mut Vec<blackbox_gfx::TexturedEffect>,
    ) {
        for actor in self.water.actors.values() {
            actor.geometry(forward, textures, out);
        }
    }
}
