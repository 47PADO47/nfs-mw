//! One assembled car on a floor, with an orbit camera (or a free camera, from the console).

mod commands;
mod floor;
pub(crate) mod materials;

use std::collections::HashMap;

use anyhow::Result;
use blackbox_render::{FrameParams, Instance, MeshHandle, Renderer, Shading};
use blackbox_scene::{Aabb, upload_solid};
use game_install::GameDir;
use glam::{Mat4, Vec3};
use nfsmw_data::car::{CarModel, LoadOptions};

use crate::input::ActionState;
use crate::viewer::Scene;
use crate::viewer::camera::{FlyCamera, OrbitCamera};
use materials::CarMaterials;

/// Where cars come from, so the console can switch to another one.
struct Source {
    dir: GameDir,
    options: LoadOptions,
}

pub struct CarScene {
    model: CarModel,
    camera: OrbitCamera,
    /// The free camera, while the console has switched to it.
    free: Option<FlyCamera>,
    source: Option<Source>,
    instances: Vec<Instance>,
    /// Everything uploaded for the current car, to free when another replaces it.
    meshes: Vec<MeshHandle>,
    materials: Option<CarMaterials>,
}

impl CarScene {
    pub fn new(model: CarModel, yaw_degrees: f32) -> Self {
        let camera = orbit_for(&model, yaw_degrees.to_radians());
        Self { model, camera, free: None, source: None, instances: Vec::new(), meshes: Vec::new(), materials: None }
    }

    /// Let the console load other cars from `dir`, with the same options.
    pub fn with_source(mut self, dir: GameDir, options: LoadOptions) -> Self {
        self.source = Some(Source { dir, options });
        self
    }

    /// Car space to world: the car stands on the floor at z = 0.
    fn to_world(&self) -> Mat4 {
        Mat4::from_translation(Vec3::Z * self.model.floor_height)
    }

    /// Free what the current car uploaded.
    fn release(&mut self, renderer: &mut Renderer) {
        for mesh in self.meshes.drain(..) {
            renderer.destroy_mesh(mesh);
        }
        if let Some(materials) = self.materials.take() {
            materials.destroy(renderer);
        }
        self.instances.clear();
    }

    /// Upload the car and its floor and list the instances.
    fn upload(&mut self, renderer: &mut Renderer) {
        let materials = CarMaterials::upload(renderer, &self.model.textures);
        let mut cache: HashMap<(u32, bool), Option<MeshHandle>> = HashMap::new();
        let world = self.to_world();
        for p in &self.model.placements {
            let mesh = *cache.entry((p.solid, p.left_brake)).or_insert_with(|| {
                let lookup = materials.for_placement(&self.model.swaps, p.left_brake);
                upload_solid(renderer, &self.model.solids[&p.solid], &lookup, Shading::Lit)
            });
            if let Some(mesh) = mesh {
                self.instances.push(Instance { mesh, transform: world * p.transform });
            }
        }
        self.meshes.extend(cache.into_values().flatten());
        if self.model.car_type.is_some() {
            let floor = floor::upload(renderer);
            self.meshes.push(floor);
            self.instances.push(Instance { mesh: floor, transform: Mat4::IDENTITY });
        }
        self.materials = Some(materials);
        // The renderer wants instances of one mesh next to each other.
        self.instances.sort_by_key(|i| i.mesh);
    }

    /// Swap in another car, keeping the viewing angle.
    fn replace_model(&mut self, renderer: &mut Renderer, model: CarModel) {
        self.release(renderer);
        let yaw = self.camera.yaw;
        self.model = model;
        self.camera = orbit_for(&self.model, yaw);
        self.upload(renderer);
    }
}

fn orbit_for(model: &CarModel, yaw: f32) -> OrbitCamera {
    let bounds = bounds(model);
    let radius = (bounds.max - bounds.min).length() * 0.5;
    OrbitCamera { target: bounds.center(), distance: radius * 2.2, yaw, pitch: 0.35 }
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

    fn captures_mouse(&self) -> bool {
        self.free.is_some()
    }

    fn init(&mut self, renderer: &mut Renderer) -> Result<()> {
        self.upload(renderer);
        Ok(())
    }

    fn update(&mut self, _renderer: &mut Renderer, input: &ActionState, dt: f32) {
        match &mut self.free {
            Some(free) => free.update(input, dt),
            None => self.camera.update(input),
        }
    }

    fn frame(&mut self, aspect: f32) -> (FrameParams, &[Instance]) {
        let (view_proj, camera_position) = match &self.free {
            Some(free) => (free.view_proj(aspect), free.position),
            None => (self.camera.view_proj(aspect), self.camera.eye()),
        };
        let params = FrameParams {
            view_proj,
            camera_position,
            light_dir: Vec3::new(-0.4, -0.3, -1.0),
            clear_color: [0.18, 0.2, 0.24],
            fog_start: f32::MAX,
            fog_end: f32::MAX,
        };
        (params, &self.instances)
    }

    fn commands(&self) -> &'static [(&'static str, &'static str)] {
        commands::LIST
    }

    fn command(&mut self, renderer: &mut Renderer, name: &str, args: &[&str]) -> Option<Result<String, String>> {
        commands::run(self, renderer, name, args)
    }
}
