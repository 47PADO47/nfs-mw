//! One assembled car on a floor, with an orbit camera.

mod floor;
mod materials;

use std::collections::HashMap;

use anyhow::Result;
use blackbox_render::{FrameParams, Instance, MeshHandle, Renderer, Shading};
use blackbox_scene::{Aabb, upload_solid};
use glam::{Mat4, Vec3};
use nfsmw_data::car::CarModel;

use crate::input::ActionState;
use crate::viewer::{Scene, camera::OrbitCamera};
use materials::CarMaterials;

pub struct CarScene {
    model: CarModel,
    camera: OrbitCamera,
    instances: Vec<Instance>,
}

impl CarScene {
    pub fn new(model: CarModel, yaw_degrees: f32) -> Self {
        let bounds = bounds(&model);
        let radius = (bounds.max - bounds.min).length() * 0.5;
        let camera =
            OrbitCamera { target: bounds.center(), distance: radius * 2.2, yaw: yaw_degrees.to_radians(), pitch: 0.35 };
        Self { model, camera, instances: Vec::new() }
    }

    /// Car space to world: the car stands on the floor at z = 0.
    fn to_world(&self) -> Mat4 {
        Mat4::from_translation(Vec3::Z * self.model.floor_height)
    }
}

/// The placed car's bounds, from each solid's box.
fn bounds(model: &CarModel) -> Aabb {
    let mut bounds = Aabb::EMPTY;
    let lift = Mat4::from_translation(Vec3::Z * model.floor_height);
    for p in &model.placements {
        let s = &model.solids[&p.solid];
        let (min, max) = (Vec3::from(s.bounds_min), Vec3::from(s.bounds_max));
        for corner in 0..8 {
            let pick = |bit: usize, lo: f32, hi: f32| if corner & bit == 0 { lo } else { hi };
            let c = Vec3::new(pick(1, min.x, max.x), pick(2, min.y, max.y), pick(4, min.z, max.z));
            bounds.extend((lift * p.transform).transform_point3(c));
        }
    }
    bounds
}

impl Scene for CarScene {
    fn title(&self) -> String {
        let paint = self.model.paint.as_ref().map(|p| format!(" — {} {}", p.name, p.hex())).unwrap_or_default();
        format!("nfsmw — {}{paint}", self.model.name)
    }

    fn init(&mut self, renderer: &mut Renderer) -> Result<()> {
        let materials = CarMaterials::upload(renderer, &self.model.textures);
        let mut meshes: HashMap<(u32, bool), Option<MeshHandle>> = HashMap::new();
        let world = self.to_world();
        for p in &self.model.placements {
            let mesh = *meshes.entry((p.solid, p.left_brake)).or_insert_with(|| {
                let lookup = materials.for_placement(&self.model.swaps, p.left_brake);
                upload_solid(renderer, &self.model.solids[&p.solid], &lookup, Shading::Lit)
            });
            if let Some(mesh) = mesh {
                self.instances.push(Instance { mesh, transform: world * p.transform });
            }
        }
        if self.model.car_type.is_some() {
            self.instances.push(Instance { mesh: floor::upload(renderer), transform: Mat4::IDENTITY });
        }
        // The renderer wants instances of one mesh next to each other.
        self.instances.sort_by_key(|i| i.mesh);
        Ok(())
    }

    fn update(&mut self, _renderer: &mut Renderer, input: &ActionState, _dt: f32) {
        self.camera.update(input);
    }

    fn frame(&mut self, aspect: f32) -> (FrameParams, &[Instance]) {
        let params = FrameParams {
            view_proj: self.camera.view_proj(aspect),
            camera_position: self.camera.eye(),
            light_dir: Vec3::new(-0.4, -0.3, -1.0),
            clear_color: [0.18, 0.2, 0.24],
            fog_start: f32::MAX,
            fog_end: f32::MAX,
        };
        (params, &self.instances)
    }
}
