//! The car as drawable parts, posed from the physics state each frame.

use std::collections::HashMap;

use blackbox_gfx::{Instance, MeshHandle, RenderBackend, Shading};
use blackbox_scene::upload_solid;
use glam::{Mat4, Quat, Vec3};
use nfsmw_data::car::{CarModel, TextureSwaps, WheelPose};

use crate::scenes::car::materials::CarMaterials;
use crate::scenes::world::car_lights::CarLights;
use crate::scenes::world::keys;
use crate::settings::CarShading;

/// Where the car is and how its wheels sit, in the world.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarPose {
    /// The car model's origin in the world (render space: x forward, y left, z up).
    pub position: Vec3,
    /// The car model's frame in the world.
    pub rotation: Quat,
    /// The wheels in the model's order: front left, front right, rear right, rear left.
    pub wheels: [WheelPose; 4],
}

impl CarPose {
    /// Between two physics steps, `t` of the way from `self` to `next`.
    pub fn lerp(&self, next: &CarPose, t: f32) -> CarPose {
        let mix = |a: f32, b: f32| a + (b - a) * t;
        CarPose {
            position: self.position.lerp(next.position, t),
            rotation: self.rotation.slerp(next.rotation, t),
            wheels: std::array::from_fn(|i| {
                let (a, b) = (self.wheels[i], next.wheels[i]);
                WheelPose { steer: mix(a.steer, b.steer), spin: mix(a.spin, b.spin), travel: mix(a.travel, b.travel) }
            }),
        }
    }

    pub fn transform(&self) -> Mat4 {
        Mat4::from_rotation_translation(self.rotation, self.position)
    }
}

/// One uploaded part: which placement of the model it is.
struct Part {
    mesh: MeshHandle,
    placement: usize,
}

pub struct CarRig {
    /// Which car this is, for the instance keys.
    car: u32,
    model: CarModel,
    /// Sorted by mesh, as the renderer wants instances of one mesh together.
    parts: Vec<Part>,
    meshes: Vec<MeshHandle>,
    materials: CarMaterials,
    shading: CarShading,
    lights: CarLights,
    /// The model scale (`CarScale`); 1 is the original size. Only the body parts scale, not the lights.
    scale: f32,
    /// The impact zones whose glass is cracked (their windows show the cracked texture).
    cracked_zones: Vec<usize>,
}

impl CarRig {
    /// Upload `model` with `shading`, lit by a sun in direction `to_sun` from the car.
    pub fn upload(
        renderer: &mut dyn RenderBackend,
        car: u32,
        model: CarModel,
        shading: CarShading,
        to_sun: Vec3,
    ) -> Self {
        let swaps = model.swaps.clone();
        let (parts, meshes, materials) = upload_parts(renderer, &model, &swaps, shading, to_sun);
        let lights = CarLights::for_model(&model);
        Self { car, model, parts, meshes, materials, shading, lights, scale: 1.0, cracked_zones: Vec::new() }
    }

    /// Upload the car again with `shading` when it differs from the one it has.
    pub fn reshade(&mut self, renderer: &mut dyn RenderBackend, shading: CarShading, to_sun: Vec3) {
        if shading == self.shading {
            return;
        }
        for mesh in self.meshes.drain(..) {
            renderer.destroy_mesh(mesh);
        }
        let swaps = self.swaps();
        let (parts, meshes, materials) = upload_parts(renderer, &self.model, &swaps, shading, to_sun);
        std::mem::replace(&mut self.materials, materials).destroy(renderer);
        (self.parts, self.meshes, self.shading) = (parts, meshes, shading);
    }

    /// Scale the model about its origin (`CarScale`).
    pub fn set_scale(&mut self, scale: f32) {
        self.scale = scale;
    }

    /// The texture swaps of the car with the cracked windows applied: the body's window placeholders of those
    /// zones, and the diffuse textures of their window solids.
    fn swaps(&self) -> TextureSwaps {
        let mut hashes = Vec::new();
        for &zone in &self.cracked_zones {
            hashes.extend(nfsmw_data::car::damage::zone_windows(zone).iter().map(|w| blackbox_hash::bstring_hash(w)));
            let Some(suffix) = nfsmw_data::car::damage::zone_window_solid(zone) else { continue };
            for solid in self.model.solids.values().filter(|s| s.name.ends_with(suffix)) {
                hashes.extend(solid.groups.iter().filter_map(|g| g.diffuse_texture(solid)));
            }
        }
        self.model.swaps.clone().with_window_hashes(&hashes)
    }

    /// Show the cracked-glass texture on the windows of these zones (see `zone_windows`). The car is
    /// re-uploaded only when the set changes.
    pub fn set_cracked_zones(&mut self, renderer: &mut dyn RenderBackend, zones: Vec<usize>, to_sun: Vec3) {
        if zones == self.cracked_zones {
            return;
        }
        self.cracked_zones = zones;
        for mesh in self.meshes.drain(..) {
            renderer.destroy_mesh(mesh);
        }
        let swaps = self.swaps();
        let (parts, meshes, materials) = upload_parts(renderer, &self.model, &swaps, self.shading, to_sun);
        std::mem::replace(&mut self.materials, materials).destroy(renderer);
        (self.parts, self.meshes) = (parts, meshes);
    }

    /// The lamps of the car, as glows.
    pub fn lights(&self) -> &CarLights {
        &self.lights
    }

    /// The model the rig was uploaded from.
    pub fn model(&self) -> &CarModel {
        &self.model
    }

    pub fn release(self, renderer: &mut dyn RenderBackend) {
        for mesh in self.meshes {
            renderer.destroy_mesh(mesh);
        }
        self.materials.destroy(renderer);
    }

    /// Height of each wheel's centre above the model's origin at rest (the model's wheel order).
    pub fn rest_heights(&self) -> [f32; 4] {
        match &self.model.corners {
            Some(corners) => std::array::from_fn(|i| corners[i].centre().z),
            None => [0.0; 4],
        }
    }

    pub(super) fn visual_tires(&self) -> Option<[super::visual_tires::VisualTire; 4]> {
        super::visual_tires::from_model(&self.model)
    }

    /// Append the car's instances for `pose`. Slots in `hidden` are skipped; corner damage overlays
    /// (`DAMAGE0_*`) are drawn only when their slot is in `revealed`.
    pub fn instances(
        &self,
        pose: &CarPose,
        hidden: Option<&std::collections::BTreeSet<usize>>,
        revealed: &std::collections::BTreeSet<usize>,
        out: &mut Vec<Instance>,
    ) {
        let world = pose.transform() * Mat4::from_scale(Vec3::splat(self.scale));
        let posed: Option<[(Mat4, Mat4); 4]> =
            self.model.corners.as_ref().map(|c| std::array::from_fn(|i| c[i].posed(pose.wheels[i])));
        for part in &self.parts {
            let placement = &self.model.placements[part.placement];
            if hidden.is_some_and(|hidden| hidden.contains(&placement.slot)) {
                continue;
            }
            if nfsmw_data::car::damage::is_corner_overlay(placement.slot) && !revealed.contains(&placement.slot) {
                continue;
            }
            let local = match (placement.corner, &posed) {
                (Some(corner), Some(posed)) => {
                    let (wheel, brake) = posed[corner];
                    if placement.is_brake() { brake } else { wheel }
                }
                _ => placement.transform,
            };
            out.push(Instance::keyed(part.mesh, world * local, keys::car_part(self.car, part.placement)));
        }
    }

    pub fn fragment_instances(&self, slot: usize) -> Vec<Instance> {
        self.parts
            .iter()
            .filter_map(|part| {
                let placement = &self.model.placements[part.placement];
                (placement.slot == slot).then_some(Instance::keyed(
                    part.mesh,
                    placement.transform,
                    keys::car_part(self.car, part.placement),
                ))
            })
            .collect()
    }
}

impl super::Drive {
    /// Draw the car with `shading` from now on (it is uploaded again when that changes).
    pub fn set_car_shading(&mut self, renderer: &mut dyn RenderBackend, shading: CarShading, to_sun: Vec3) {
        self.rig.reshade(renderer, shading, to_sun);
    }
}

/// Upload the placed solids of `model` with `shading`: the parts sorted by mesh (the renderer wants instances
/// of one mesh together), every mesh uploaded and the car's materials.
fn upload_parts(
    renderer: &mut dyn RenderBackend,
    model: &CarModel,
    swaps: &TextureSwaps,
    shading: CarShading,
    to_sun: Vec3,
) -> (Vec<Part>, Vec<MeshHandle>, CarMaterials) {
    let materials = CarMaterials::upload(renderer, model, shading, to_sun);
    let mut cache: HashMap<(u32, bool), Option<MeshHandle>> = HashMap::new();
    let mut parts = Vec::new();
    for (placement, p) in model.placements.iter().enumerate() {
        let mesh = *cache.entry((p.solid, p.left_brake)).or_insert_with(|| {
            let lookup = materials.for_placement(swaps, p.left_brake);
            upload_solid(renderer, &model.solids[&p.solid], &lookup, Shading::Lit)
        });
        if let Some(mesh) = mesh {
            parts.push(Part { mesh, placement });
        }
    }
    parts.sort_by_key(|p| p.mesh);
    (parts, cache.into_values().flatten().collect(), materials)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pose(x: f32, yaw: f32) -> CarPose {
        CarPose {
            position: Vec3::new(x, 0.0, 0.0),
            rotation: Quat::from_rotation_z(yaw),
            wheels: [WheelPose { steer: x, spin: x * 2.0, travel: 0.0 }; 4],
        }
    }

    #[test]
    fn interpolates_position_rotation_and_wheels() {
        let mid = pose(0.0, 0.0).lerp(&pose(10.0, 1.0), 0.5);
        assert!((mid.position.x - 5.0).abs() < 1e-5);
        assert!((mid.rotation.to_euler(glam::EulerRot::ZYX).0 - 0.5).abs() < 1e-5);
        assert!((mid.wheels[2].steer - 5.0).abs() < 1e-5 && (mid.wheels[3].spin - 10.0).abs() < 1e-5);
    }

    #[test]
    fn transform_moves_the_model() {
        let p = pose(3.0, std::f32::consts::FRAC_PI_2);
        let moved = p.transform().transform_point3(Vec3::X);
        assert!((moved - Vec3::new(3.0, 1.0, 0.0)).length() < 1e-5);
    }
}
