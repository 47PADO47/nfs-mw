//! The car as drawable parts, posed from the physics state each frame.

use std::collections::HashMap;

use blackbox_render::{Instance, MeshHandle, Renderer, Shading};
use blackbox_scene::upload_solid;
use glam::{Mat4, Quat, Vec3};
use nfsmw_data::car::{CarModel, WheelPose};

use crate::scenes::car::materials::CarMaterials;

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
    model: CarModel,
    /// Sorted by mesh, as the renderer wants instances of one mesh together.
    parts: Vec<Part>,
    meshes: Vec<MeshHandle>,
    materials: CarMaterials,
}

impl CarRig {
    pub fn upload(renderer: &mut Renderer, model: CarModel) -> Self {
        let materials = CarMaterials::upload(renderer, &model.textures);
        let mut cache: HashMap<(u32, bool), Option<MeshHandle>> = HashMap::new();
        let mut parts = Vec::new();
        for (placement, p) in model.placements.iter().enumerate() {
            let mesh = *cache.entry((p.solid, p.left_brake)).or_insert_with(|| {
                let lookup = materials.for_placement(&model.swaps, p.left_brake);
                upload_solid(renderer, &model.solids[&p.solid], &lookup, Shading::Lit)
            });
            if let Some(mesh) = mesh {
                parts.push(Part { mesh, placement });
            }
        }
        parts.sort_by_key(|p| p.mesh);
        let meshes = cache.into_values().flatten().collect();
        Self { model, parts, meshes, materials }
    }

    /// The model the rig was uploaded from.
    pub fn model(&self) -> &CarModel {
        &self.model
    }

    pub fn release(self, renderer: &mut Renderer) {
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

    /// Append the car's instances for `pose`.
    pub fn instances(&self, pose: &CarPose, out: &mut Vec<Instance>) {
        let world = pose.transform();
        let posed: Option<[(Mat4, Mat4); 4]> =
            self.model.corners.as_ref().map(|c| std::array::from_fn(|i| c[i].posed(pose.wheels[i])));
        for part in &self.parts {
            let placement = &self.model.placements[part.placement];
            let local = match (placement.corner, &posed) {
                (Some(corner), Some(posed)) => {
                    let (wheel, brake) = posed[corner];
                    if placement.is_brake() { brake } else { wheel }
                }
                _ => placement.transform,
            };
            out.push(Instance { mesh: part.mesh, transform: world * local });
        }
    }
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
