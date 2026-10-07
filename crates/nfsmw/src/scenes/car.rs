//! One car with an orbit camera.

use std::collections::HashMap;

use anyhow::Result;
use blackbox_render::{BlendMode, FrameParams, Instance, Renderer, Shading, TextureHandle};
use blackbox_scene::{Aabb, blend_mode, upload_solid, upload_texture};
use glam::{Mat4, Vec3};
use nfsmw_data::car::CarModel;

use crate::viewer::{Input, Scene, camera::OrbitCamera};

pub struct CarScene {
    model: CarModel,
    camera: OrbitCamera,
    instances: Vec<Instance>,
}

impl CarScene {
    pub fn new(model: CarModel, yaw_degrees: f32) -> Self {
        let mut bounds = Aabb::EMPTY;
        for v in model.solids.iter().flat_map(|s| &s.vertices) {
            bounds.extend(Vec3::from(v.position));
        }
        let radius = (bounds.max - bounds.min).length() * 0.5;
        let camera =
            OrbitCamera { target: bounds.center(), distance: radius * 2.2, yaw: yaw_degrees.to_radians(), pitch: 0.35 };
        Self { model, camera, instances: Vec::new() }
    }
}

impl Scene for CarScene {
    fn title(&self) -> String {
        format!("nfsmw — {}", self.model.name)
    }

    fn init(&mut self, renderer: &mut Renderer) -> Result<()> {
        let mut materials: HashMap<u32, (TextureHandle, BlendMode)> = HashMap::new();
        for (&hash, t) in &self.model.textures {
            if let Some(handle) = upload_texture(renderer, t) {
                materials.insert(hash, (handle, blend_mode(Some(t))));
            }
        }
        let lookup = |hash: u32| materials.get(&hash).copied();
        for solid in &self.model.solids {
            if let Some(mesh) = upload_solid(renderer, solid, &lookup, Shading::Lit) {
                self.instances.push(Instance { mesh, transform: Mat4::IDENTITY });
            }
        }
        Ok(())
    }

    fn update(&mut self, _renderer: &mut Renderer, input: &Input, _dt: f32) {
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
