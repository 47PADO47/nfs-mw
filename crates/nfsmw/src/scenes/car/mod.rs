//! One assembled car on a floor, with an orbit camera (or a free camera, from the console).

mod commands;
mod floor;
#[cfg(test)]
mod lifecycle_tests;
pub(crate) mod lighting;
mod material_response;
pub(crate) mod materials;
pub(crate) mod original_lighting;
mod paint_surface;
mod shading;
pub(crate) mod shadows;
mod texture_anim;
mod wheels;

use std::collections::HashMap;

use anyhow::Result;
use blackbox_gfx::{FrameParams, Instance, InstanceKey, MeshHandle, RenderBackend, Shading};
use blackbox_scene::{Aabb, upload_solid};
use game_install::GameDir;
use glam::{Mat4, Vec3};
use nfsmw_data::car::{CarModel, LoadOptions};

use crate::input::ActionState;
use crate::settings::CarShading;
use crate::viewer::Scene;
use crate::viewer::camera::{Camera, CameraCut, FlyCamera, OrbitCamera};
use materials::CarMaterials;

/// The direction the light travels in the viewer (the placeholder shading of the floor uses it
/// directly; cars get a rig built around the sun it comes from).
const LIGHT_DIR: Vec3 = Vec3::new(-0.4, -0.3, -1.0);

/// The floor's part number in the instance keys (cars have fewer parts than this).
pub(crate) const FLOOR_PART: u32 = u32::MAX - 1;

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
    /// The car shading the current car was uploaded with, and the one asked for since.
    shading: CarShading,
    wanted_shading: CarShading,
    /// Authored presets sampled at this viewer's fixed sun and clear-cloud preview context.
    preview_lighting: Option<nfsmw_data::time_of_day::Presets>,
    /// The installed shop backroom mirrors the front end's light angles (`car-lighting.md`).
    frontend_backroom: bool,
    /// Which car is on the floor: another car gets new instance keys.
    car: u32,
    /// The camera jumped (a new scene, another car, the camera toggle).
    cut: CameraCut,
    /// Where the car sits across the picture, in normalised device units: 0 keeps it in the middle, 1 moves it
    /// by a full half-width to the right. A viewer with a panel on the left uses the middle of the free part.
    pub(crate) view_offset: f32,
    /// How the solids are lit: lit for cars; a scenery model is prelit (its colours are in the vertices).
    pub(crate) solid_shading: Shading,
    /// Texture animations of the model on the floor (scenery only).
    pub(crate) texture_anims: Vec<blackbox_tpk::TextureAnim>,
    /// Whether the plain placeholder floor is drawn under a car. A scene with its own floor turns it off, since
    /// two coplanar floors flicker where they overlap.
    pub(crate) placeholder_floor: bool,
    /// How the car is turned on its floor and how far its tyres sink into it (`CarScene::stand_on`).
    stand: CarStand,
}

/// A car's turn about the vertical axis and how far it is lowered from resting on the floor.
#[derive(Clone, Copy, Default)]
struct CarStand {
    yaw: f32,
    sink: f32,
}

impl CarScene {
    pub fn new(model: CarModel, yaw_degrees: f32) -> Self {
        let camera = orbit_for(&model, yaw_degrees.to_radians());
        Self {
            model,
            camera,
            free: None,
            source: None,
            instances: Vec::new(),
            meshes: Vec::new(),
            materials: None,
            shading: CarShading::default(),
            wanted_shading: CarShading::default(),
            preview_lighting: None,
            frontend_backroom: false,
            car: 0,
            cut: CameraCut::default(),
            view_offset: 0.0,
            solid_shading: Shading::Lit,
            texture_anims: Vec::new(),
            placeholder_floor: true,
            stand: CarStand::default(),
        }
    }

    /// Turn the car `yaw_degrees` about the vertical axis and lower it `sink` metres into the floor, as a front-end stage
    /// stands it. Takes effect when the car is uploaded.
    pub(crate) fn stand_on(&mut self, yaw_degrees: f32, sink: f32) {
        self.stand = CarStand { yaw: yaw_degrees.to_radians(), sink };
    }

    /// The orbit camera, for a scene that moves it (the menus' camera move).
    pub(crate) fn camera_mut(&mut self) -> &mut OrbitCamera {
        &mut self.camera
    }

    /// Let the console load other cars from `dir`, with the same options.
    pub fn with_source(mut self, dir: GameDir, options: LoadOptions) -> Self {
        // The session's parsed attributes (read once, see `nfsmw_data::shared`), not a read of the file per backdrop.
        self.preview_lighting =
            nfsmw_data::shared::attributes(&dir).ok().and_then(|db| nfsmw_data::time_of_day::load(&db));
        self.source = Some(Source { dir, options });
        self
    }

    fn update_original_lighting(&self, renderer: &mut dyn RenderBackend) {
        if !self.shading.is_original() {
            return;
        }
        let scale = self
            .preview_lighting
            .as_ref()
            .and_then(|presets| nfsmw_data::time_of_day::car_specular_scale(presets, 0.0, (-LIGHT_DIR).normalize().z))
            .unwrap_or(1.0);
        let rig = match self.frontend_backroom {
            true => original_lighting::backroom(scale),
            false => original_lighting::frontend(scale),
        };
        renderer.set_lighting_rig(&rig);
    }

    /// Select the original backroom rig without changing a custom shading mode or material profile.
    pub(crate) fn set_frontend_backroom(&mut self, backroom: bool) {
        self.frontend_backroom = backroom;
    }

    /// Car space to world: the car stands on the floor at z = 0, less what it sinks in, turned by its stand.
    fn to_world(&self) -> Mat4 {
        Mat4::from_translation(Vec3::Z * (self.model.floor_height - self.stand.sink))
            * Mat4::from_rotation_z(self.stand.yaw)
    }

    /// Free what the current car uploaded.
    pub(crate) fn release(&mut self, renderer: &mut dyn RenderBackend) {
        for mesh in self.meshes.drain(..) {
            renderer.destroy_mesh(mesh);
        }
        if let Some(materials) = self.materials.take() {
            materials.destroy(renderer);
        }
        self.instances.clear();
    }

    /// The car is in the window again after another scene had it: its light rig is put back (the other scene set
    /// its own), and the next frame is a camera cut, so nothing is blended from the other scene's picture.
    pub(crate) fn reshow(&mut self, renderer: &mut dyn RenderBackend) {
        materials::set_lighting(renderer, self.shading, -LIGHT_DIR);
        self.cut.arm();
    }

    /// Upload the car and its floor and list the instances.
    fn upload(&mut self, renderer: &mut dyn RenderBackend) {
        self.shading = self.wanted_shading;
        let materials = CarMaterials::upload(renderer, &self.model, self.shading, -LIGHT_DIR);
        let mut swaps = match materials.enhanced_lamps() {
            true => self.model.swaps.clone().with_lamp_state(true, false, &self.model.textures),
            false => self.model.swaps.clone(),
        };
        if materials.enhanced_lamps() {
            for kind in ["HEADLIGHT_GLASS", "BRAKELIGHT_GLASS"] {
                for side in ["LEFT", "RIGHT", "CENTRE"] {
                    let hash = blackbox_hash::bstring_hash(&format!("{kind}_{side}"));
                    swaps.map.insert(hash, self.model.swaps.resolve(hash));
                }
            }
        }
        let mut cache: HashMap<(u32, bool), Option<MeshHandle>> = HashMap::new();
        let world = self.to_world();
        for (part, p) in self.model.placements.iter().enumerate() {
            if nfsmw_data::car::damage::is_corner_overlay(p.slot) {
                continue;
            }
            let mesh = *cache.entry((p.solid, p.left_brake)).or_insert_with(|| {
                let lookup = materials.for_solid(&swaps, p.left_brake, &self.model.solids[&p.solid].name);
                upload_solid(renderer, &self.model.solids[&p.solid], &lookup, self.solid_shading)
            });
            if let Some(mesh) = mesh {
                self.instances.push(Instance::keyed(
                    mesh,
                    world * p.transform,
                    InstanceKey::new(self.car, part as u32),
                ));
            }
        }
        self.meshes.extend(cache.into_values().flatten());
        if self.model.car_type.is_some() && self.placeholder_floor {
            let floor = floor::upload(renderer);
            self.meshes.push(floor);
            self.instances.push(Instance::keyed(floor, Mat4::IDENTITY, InstanceKey::new(self.car, FLOOR_PART)));
        }
        self.materials = Some(materials);
        self.update_original_lighting(renderer);
        // The renderer wants instances of one mesh next to each other.
        self.instances.sort_by_key(|i| i.mesh);
    }

    /// Swap in another car, keeping the viewing angle.
    pub(crate) fn replace_model(&mut self, renderer: &mut dyn RenderBackend, model: CarModel) {
        self.release(renderer);
        let yaw = self.camera.yaw;
        self.car += 1;
        self.cut.arm();
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
    for p in model.placements.iter().filter(|p| !nfsmw_data::car::damage::is_corner_overlay(p.slot)) {
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
    fn close(&mut self, renderer: &mut dyn RenderBackend) {
        // The inherent helper, not this hook: method paths name it first.
        CarScene::release(self, renderer);
    }

    fn title(&self) -> String {
        let paint = self.model.paint.as_ref().map(|p| format!(" — {} {}", p.name, p.hex())).unwrap_or_default();
        format!("nfsmw — {}{paint}", self.model.name)
    }

    fn captures_mouse(&self) -> bool {
        self.free.is_some()
    }

    fn init(&mut self, renderer: &mut dyn RenderBackend) -> Result<()> {
        self.upload(renderer);
        Ok(())
    }

    fn set_car_shading(&mut self, shading: CarShading) {
        self.wanted_shading = shading;
    }

    fn update(&mut self, renderer: &mut dyn RenderBackend, input: &ActionState, dt: f32) {
        self.refresh_effects(renderer);
        match &mut self.free {
            Some(free) => free.update(input, dt),
            None => self.camera.update(input),
        }
    }

    fn refresh_effects(&mut self, renderer: &mut dyn RenderBackend) {
        if self.wanted_shading != self.shading {
            self.release(renderer);
            self.upload(renderer);
        }
        self.update_original_lighting(renderer);
    }

    fn frame(&mut self, aspect: f32) -> (FrameParams, &[Instance]) {
        let (mut params, _) = self.snapshot(aspect);
        params.camera_cut = self.cut.take();
        (params, &self.instances)
    }

    fn commands(&self) -> &'static [(&'static str, &'static str)] {
        commands::LIST
    }

    fn command(
        &mut self,
        renderer: &mut dyn RenderBackend,
        name: &str,
        args: &[&str],
    ) -> Option<Result<String, String>> {
        commands::run(self, renderer, name, args)
    }
}

mod snapshot;
