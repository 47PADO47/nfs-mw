//! The car as drawable parts, posed from the physics state each frame.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use blackbox_gfx::{Instance, MeshHandle, RenderBackend, Shading};
use blackbox_scene::upload_solid;
use glam::{Mat4, Quat, Vec3};
use nfsmw_data::car::{CarModel, TextureSwaps, WheelPose};

use crate::scenes::car::materials::{CarMaterials, set_lighting};
use crate::scenes::world::car_lights::CarLights;
use crate::scenes::world::keys;
#[cfg(test)]
mod damage_gpu_tests;
#[cfg(test)]
mod damage_tests;
#[cfg(test)]
mod detail_tests;
mod glass;
pub use glass::SurfaceDamage;
#[cfg(test)]
mod surface_tests;
mod upload;
use crate::settings::CarShading;
use upload::{upload_parts, upload_solids};

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
    lamps: [MeshHandle; 4],
    placement: nfsmw_data::car::Placement,
    key: usize,
    lod: usize,
    damaged: bool,
}

/// Shared actors retain identity and physics while their uploaded appearance changes.
struct Appearance {
    parts: Vec<Part>,
    /// Every uploaded mesh, with the solid (by name hash) it draws.
    meshes: Vec<(u32, MeshHandle)>,
    materials: CarMaterials,
    shading: CarShading,
}

impl Appearance {
    fn release(self, renderer: &mut dyn RenderBackend) {
        for (_, mesh) in self.meshes {
            renderer.destroy_mesh(mesh);
        }
        self.materials.destroy(renderer);
    }
}

pub struct CarRig {
    detail_radius: f32,
    /// Default instance identity; actors that share the uploaded rig supply their own identity.
    car: u32,
    model: CarModel,
    /// Sorted by mesh, as the renderer wants instances of one mesh together.
    appearance: RefCell<Appearance>,
    lights: CarLights,
    flares: super::super::car_light_flares::CarLightFlares,
    /// The model scale (`CarScale`); 1 is the original size. Only the body parts scale, not the lights.
    scale: f32,
    glass: RefCell<std::collections::BTreeMap<SurfaceDamage, glass::GlassVariant>>,
}

impl CarRig {
    /// Reuse the loaded geometry while a color picker is dragged.
    pub fn recolor(&mut self, renderer: &mut dyn RenderBackend, rgb: [u8; 3], sun: Vec3) -> anyhow::Result<()> {
        self.model.set_body_color(rgb)?;
        self.clear_glass(renderer);
        let shading = self.appearance.get_mut().shading;
        let (parts, meshes, materials) = upload_parts(renderer, &self.model, &self.swaps(), shading, sun);
        std::mem::replace(self.appearance.get_mut(), Appearance { parts, meshes, materials, shading })
            .release(renderer);
        Ok(())
    }
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
        let flares = super::super::car_light_flares::CarLightFlares::upload(renderer, &model);
        Self {
            detail_radius: super::car_detail::radius(&model),
            car,
            model,
            appearance: RefCell::new(Appearance { parts, meshes, materials, shading }),
            lights,
            flares,
            scale: 1.0,
            glass: RefCell::default(),
        }
    }

    /// Upload the car again with `shading` when it differs from the one it has.
    pub fn reshade(
        &self,
        renderer: &mut dyn RenderBackend,
        shading: CarShading,
        to_sun: Vec3,
    ) -> Vec<(MeshHandle, MeshHandle)> {
        if shading == self.appearance.borrow().shading {
            return Vec::new();
        }
        self.clear_glass(renderer);
        let swaps = self.swaps();
        let (parts, meshes, materials) = upload_parts(renderer, &self.model, &swaps, shading, to_sun);
        let mut appearance = self.appearance.borrow_mut();
        let replacements = appearance
            .parts
            .iter()
            .filter_map(|old| {
                parts
                    .iter()
                    .find(|new| new.key == old.key && new.lod == old.lod && new.damaged == old.damaged)
                    .map(|new| (old.mesh, new.mesh))
            })
            .collect();
        std::mem::replace(&mut *appearance, Appearance { parts, meshes, materials, shading }).release(renderer);
        replacements
    }

    /// Scale the model about its origin (`CarScale`).
    pub fn set_scale(&mut self, scale: f32) {
        self.scale = scale;
    }

    fn swaps(&self) -> TextureSwaps {
        self.model.swaps.clone()
    }

    /// The lamps of the car, as glows.
    pub fn lights(&self) -> &CarLights {
        &self.lights
    }

    pub fn flares(&self) -> &super::super::car_light_flares::CarLightFlares {
        &self.flares
    }

    /// The model the rig was uploaded from.
    pub fn model(&self) -> &CarModel {
        &self.model
    }

    pub fn release(self, renderer: &mut dyn RenderBackend) {
        self.clear_glass(renderer);
        self.appearance.into_inner().release(renderer);
        self.flares.release(renderer);
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
        self.instances_for(self.car, pose, hidden, revealed, out);
    }

    /// Draw one physical vehicle while retaining the rig's shared meshes and materials.
    pub fn instances_for(
        &self,
        car: u32,
        pose: &CarPose,
        hidden: Option<&std::collections::BTreeSet<usize>>,
        revealed: &std::collections::BTreeSet<usize>,
        out: &mut Vec<Instance>,
    ) {
        self.instances_lit(car, pose, hidden, revealed, [true, false], out);
    }

    pub fn instances_lit(
        &self,
        car: u32,
        pose: &CarPose,
        hidden: Option<&std::collections::BTreeSet<usize>>,
        revealed: &std::collections::BTreeSet<usize>,
        lamps: [bool; 2],
        out: &mut Vec<Instance>,
    ) {
        self.instances_detail(pose, hidden, revealed, super::DrawOptions { car, lamps, ..Default::default() }, out);
    }

    pub fn instances_detail(
        &self,
        pose: &CarPose,
        hidden: Option<&std::collections::BTreeSet<usize>>,
        revealed: &std::collections::BTreeSet<usize>,
        draw: super::DrawOptions<'_>,
        out: &mut Vec<Instance>,
    ) {
        let super::DrawOptions { car, lamps, detail, glass, broken_lights, damage } = draw;
        let damage_slots = damage
            .filter(|_| self.model.driving_lods.damage.enabled)
            .map_or(0, nfsmw_data::car::damage_parts::damaged_slots);
        let lod = detail.map_or(0, |view| {
            super::car_detail::choose(
                &self.model,
                pose.position,
                self.detail_radius * self.scale,
                view,
                car == keys::PLAYER_CAR,
            )
        });
        let state = usize::from(lamps[0]) | (usize::from(lamps[1]) << 1);
        let world = pose.transform() * Mat4::from_scale(Vec3::splat(self.scale));
        let posed: Option<[(Mat4, Mat4); 4]> =
            self.model.corners.as_ref().map(|c| std::array::from_fn(|i| c[i].posed(pose.wheels[i])));
        let variants = self.glass.borrow();
        let variant = variants.get(&SurfaceDamage::new(glass, broken_lights));
        for base in &self.appearance.borrow().parts {
            // A glass variant keeps its part's key, LOD and damage flag (`glass.rs` keys it by them), so the other
            // LODs are skipped before the variant lookup.
            if base.lod != lod {
                continue;
            }
            let part = variant.and_then(|v| v.parts.get(&(base.key, base.lod, base.damaged))).unwrap_or(base);
            if part.lod != lod {
                continue;
            }
            let placement = &part.placement;
            let damaged = placement.slot <= 22 && damage_slots & (1 << placement.slot) != 0;
            if part.damaged != damaged {
                continue;
            }
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
            out.push(Instance::keyed(part.lamps[state], world * local, keys::car_part(car, part.key)));
        }
    }

    pub fn fragment_instances(&self, slot: usize, damage: Option<&[u8; 10]>) -> Vec<Instance> {
        let damaged = self.model.driving_lods.damage.enabled
            && slot <= 22
            && damage.is_some_and(|levels| nfsmw_data::car::damage_parts::damaged_slots(levels) & (1 << slot) != 0);
        self.appearance
            .borrow()
            .parts
            .iter()
            .filter_map(|part| {
                let placement = &part.placement;
                (part.lod == 0 && part.damaged == damaged && placement.slot == slot).then_some(Instance::keyed(
                    part.mesh,
                    placement.transform,
                    keys::car_part(self.car, part.key),
                ))
            })
            .collect()
    }
}

impl super::Drive {
    /// Draw the car with `shading` from now on (it is uploaded again when that changes).
    pub fn set_car_shading(
        &mut self,
        renderer: &mut dyn RenderBackend,
        shading: CarShading,
        to_sun: Vec3,
    ) -> Vec<(MeshHandle, MeshHandle)> {
        self.rig.reshade(renderer, shading, to_sun)
    }
}

/// The texture hashes whose replacement differs between `before` and `after`: the window placeholders and
/// window textures whose glass was cracked or repaired.
fn changed_textures(before: &TextureSwaps, after: &TextureSwaps) -> HashSet<u32> {
    before
        .map
        .keys()
        .chain(after.map.keys())
        .copied()
        .filter(|&hash| before.resolve(hash) != after.resolve(hash))
        .collect()
}

/// The solids (by name hash) with a group that draws one of the `changed` textures. Their meshes bake the
/// texture each group draws, so they are the only meshes a change of replacement affects.
fn affected_solids(model: &CarModel, changed: &HashSet<u32>) -> HashSet<u32> {
    model
        .solids
        .iter()
        .filter(|(_, solid)| {
            solid.groups.iter().filter_map(|g| g.diffuse_texture(solid)).any(|hash| changed.contains(&hash))
        })
        .map(|(&hash, _)| hash)
        .collect()
}

#[cfg(test)]
mod tests {
    use blackbox_hash::bstring_hash;

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

    #[test]
    fn cracking_a_window_changes_only_its_placeholder() {
        let before = TextureSwaps::for_car("BMWM3GTR");
        let after = before.clone().with_window_hashes(&[bstring_hash("WINDOW_REAR")]);
        let changed = changed_textures(&before, &after);
        assert_eq!(changed, HashSet::from([bstring_hash("WINDOW_REAR")]));
        assert_eq!(changed_textures(&after, &before), changed, "repairing the window changes the same texture");
        assert!(changed_textures(&after, &after).is_empty());
    }
}
