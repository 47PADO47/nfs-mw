//! A rider on a two-wheeler: a skinned, animated character drawn on the player's bike.
//!
//! A mod gives a bike a rider with files (`docs/formats/ride6-rider.md`; a mod's `files/CARS/<ID>/RIDER/` or `files/RIDER/`):
//! `rider.bbsk` (the skeleton, the animations and the skinned meshes), `graph.json` (the blend tree), `attach.json` (the seat,
//! the grips, the pegs, the limbs to solve and the numbers that drive the animation), `parts.json` (which picture each mesh
//! wears) and `textures/*.png`. A bike without `attach.json` has no rider; the game says so once and goes on.
//!
//! Each physics step feeds the rider the bike's state ([`Rider::step`]: the smoothed inputs of the graph and the clock). Each
//! frame the rider is posed ([`pose`]: the graph, the body's shift, the hands on the grips and the feet on the pegs), skinned
//! on the CPU and its vertices written to the meshes the renderer holds ([`Rider::maintain`]), and drawn at the bike's
//! transform ([`Rider::instances`]). Nothing here names a game: the files say what the bones are called.

mod attach;
mod params;
mod parts;
mod pose;

use std::collections::HashMap;

use anyhow::{Context, Result};
use blackbox_gfx::{DrawRange, Instance, MeshDesc, MeshHandle, RenderBackend, TextureHandle, Vertex};
use blackbox_skeleton::{Graph, Pack, SkinVertex, skin_matrices, skin_vertices};
use game_install::GameDir;
use glam::{Mat4, Vec3};
use nfsmw_data::car::CarModel;

use self::attach::Attach;
pub use self::params::RiderSample;
use self::params::{Smoothed, Values};
use self::pose::{Frame, Resolved, Steering};
use super::CarPose;
use crate::scenes::world::keys;

/// The first part number a rider's meshes take in the instance keys of the player's car (the car's own placements come
/// before it).
const KEY_BASE: usize = 0x4000;

/// Where the rider files of the car `car` are looked for, the first folder with an `attach.json` winning: the car's own, then
/// the one every bike of the mod shares.
pub fn folders(car: &str) -> [String; 2] {
    [format!("CARS/{car}/RIDER"), "RIDER".to_owned()]
}

/// The graph's parameters by name: the names are the ones the animation spec gives (`lean`, `speed_kmh`, `tuck`, `wheelie`,
/// `stoppie`); a parameter the graph has and this does not know is 0.
pub fn param_values(names: &[String], values: &Values) -> Vec<f32> {
    names
        .iter()
        .map(|name| match name.as_str() {
            "lean" => values.lean,
            "speed_kmh" => values.speed_kmh,
            "tuck" => values.tuck,
            "wheelie" => values.wheelie,
            "stoppie" => values.stoppie,
            _ => 0.0,
        })
        .collect()
}

/// The renderer's vertices for skinned positions and normals: white, with the mesh's own texture coordinates.
pub fn fill(out: &mut Vec<Vertex>, source: &[SkinVertex], positions: &[Vec3], normals: &[Vec3]) {
    out.clear();
    out.extend(source.iter().zip(positions.iter().zip(normals)).map(|(vertex, (position, normal))| Vertex {
        position: position.to_array(),
        normal: normal.to_array(),
        color_bgra: [255; 4],
        uv: vertex.uv,
    }));
}

/// One mesh of the rider on the renderer.
struct GpuPart {
    /// The mesh in `Pack::meshes` and the level of detail drawn.
    mesh: usize,
    level: usize,
    handle: MeshHandle,
    label: String,
    vertices: Vec<Vertex>,
    indices: Vec<u16>,
    draws: Vec<DrawRange>,
}

/// A loaded rider.
struct RiderRig {
    pack: Pack,
    graph: Graph,
    attach: Attach,
    resolved: Resolved,
    frame: Frame,
    /// The axis the handlebar turns about and a point on it, in the model's frame (from the front wheel's mount).
    steering: Option<(Vec3, Vec3)>,
    smoothed: Smoothed,
    /// Seconds of animation: the physics steps' time.
    time: f32,
    parts: Vec<GpuPart>,
    textures: Vec<TextureHandle>,
    matrices: Vec<Mat4>,
    positions: Vec<Vec3>,
    normals: Vec<Vec3>,
    /// Whether an error of the animation has been logged (once).
    failed: bool,
}

impl RiderRig {
    /// Read and upload the rider of `model`. `Ok(None)` when the car has no rider files.
    fn load(renderer: &mut dyn RenderBackend, dir: &GameDir, model: &CarModel) -> Result<Option<Self>> {
        let Some(folder) = folders(&model.name).into_iter().find(|f| dir.exists(&format!("{f}/attach.json"))) else {
            return Ok(None);
        };
        let read =
            |name: &str| dir.read(&format!("{folder}/{name}")).with_context(|| format!("reading {folder}/{name}"));
        let attach = Attach::parse(&read("attach.json")?)?;
        let pack = Pack::parse(&read("rider.bbsk")?).context("rider.bbsk")?;
        let graph: Graph = serde_json::from_slice(&read("graph.json")?).context("graph.json")?;
        graph.validate(&pack.clips, &pack.skeleton).context("graph.json does not fit rider.bbsk")?;
        let resolved = Resolved::new(&attach, &pack.skeleton).map_err(anyhow::Error::msg)?;
        let entries = parts::parse_entries(&read("parts.json")?)?;
        let corners = model.corners.as_ref().context("the car has no wheel mounts")?;
        let (front, rear) = (corners[0], corners[2]);
        let frame =
            Frame::new(&attach.rider_root_in_bike.transform(), attach.frame_offset(front.centre(), rear.centre()));

        let (plans, skipped) = parts::plan(&pack, &entries);
        skipped.iter().for_each(|why| log::warn!("rider: {why}"));
        let mut textures: HashMap<String, TextureHandle> = HashMap::new();
        for picture in plans.iter().flat_map(|p| &p.draws).filter_map(|d| d.picture.as_deref()) {
            if textures.contains_key(picture) {
                continue;
            }
            let decoded = read(&format!("textures/{picture}")).and_then(|bytes| parts::decode_png(&bytes));
            match decoded {
                Ok(image) => {
                    textures.insert(picture.to_owned(), parts::upload_picture(renderer, picture, &image));
                }
                Err(error) => log::warn!("rider: the picture {picture} is not drawn: {error:#}"),
            }
        }
        let lookup = |name: &str| textures.get(name).copied();
        let mut gpu_parts = Vec::new();
        for plan in plans {
            let mesh = &pack.meshes[plan.mesh];
            let lod = &mesh.lods[plan.level];
            let (positions, normals): (Vec<Vec3>, Vec<Vec3>) =
                lod.vertices.iter().map(|v| (v.position, v.normal)).unzip();
            let mut vertices = Vec::new();
            fill(&mut vertices, &lod.vertices, &positions, &normals);
            let draws = parts::draw_ranges(&plan, &lookup);
            let label = format!("rider: {}", mesh.name);
            let handle = renderer.create_mesh(&MeshDesc {
                label: &label,
                vertices: &vertices,
                tangents: None,
                secondary_uvs: None,
                indices: &plan.indices,
                draws: draws.clone(),
            });
            gpu_parts.push(GpuPart {
                mesh: plan.mesh,
                level: plan.level,
                handle,
                label,
                vertices,
                indices: plan.indices,
                draws,
            });
        }
        log::info!(
            "rider: {} meshes, {} pictures, {} bones from {folder}",
            gpu_parts.len(),
            textures.len(),
            pack.skeleton.len()
        );
        Ok(Some(Self {
            pack,
            graph,
            attach,
            resolved,
            frame,
            steering: front.steering_axis(),
            smoothed: Smoothed::default(),
            time: 0.0,
            parts: gpu_parts,
            textures: textures.into_values().collect(),
            matrices: Vec::new(),
            positions: Vec::new(),
            normals: Vec::new(),
            failed: false,
        }))
    }

    /// One physics step of `dt` seconds with the bike in state `sample`.
    fn step(&mut self, sample: &RiderSample, dt: f32) {
        self.smoothed.step(sample, &self.attach.inputs, &self.attach.inertia, dt);
        if dt.is_finite() && dt > 0.0 {
            self.time += dt.min(0.1);
        }
    }

    /// Pose the rider with the handlebar at `steer` (rad, positive left), skin it and write the vertices to the renderer.
    fn update(&mut self, renderer: &mut dyn RenderBackend, steer: f32) {
        let steering = self.steering.map(|(axis, point)| Steering { axis, point, angle: steer });
        let targets = pose::rider_targets(&self.attach, &self.frame, steering);
        let body = pose::body(&self.frame, self.smoothed.shift(), self.smoothed.rotation());
        let params = param_values(&self.graph.param_names, &self.smoothed.values());
        let skeleton = &self.pack.skeleton;
        let animated = pose::animate(
            skeleton,
            &self.pack.clips,
            &self.graph,
            (&params, self.time),
            &self.resolved,
            (&targets, body),
        );
        let component = match animated {
            Ok(component) => component,
            Err(error) => {
                if !self.failed {
                    log::warn!("rider: the pose could not be worked out: {error}");
                    self.failed = true;
                }
                return;
            }
        };
        for part in &mut self.parts {
            let mesh = &self.pack.meshes[part.mesh];
            if skin_matrices(&component, &mesh.bone_map, &mesh.inverse_bind, &mut self.matrices).is_err() {
                continue;
            }
            let source = &mesh.lods[part.level].vertices;
            skin_vertices(source, &self.matrices, &mut self.positions, &mut self.normals);
            fill(&mut part.vertices, source, &self.positions, &self.normals);
            if renderer.update_mesh_vertices(part.handle, &part.vertices) {
                continue;
            }
            // A backend that cannot rewrite a mesh gets a new one.
            renderer.destroy_mesh(part.handle);
            part.handle = renderer.create_mesh(&MeshDesc {
                label: &part.label,
                vertices: &part.vertices,
                tangents: None,
                secondary_uvs: None,
                indices: &part.indices,
                draws: part.draws.clone(),
            });
        }
    }

    /// The rider's instances for a bike at `world` (the model's frame in the world).
    fn instances(&self, world: Mat4, out: &mut Vec<Instance>) {
        let transform = world * self.frame.to_model;
        for (n, part) in self.parts.iter().enumerate() {
            out.push(Instance::keyed(part.handle, transform, keys::car_part(keys::PLAYER_CAR, KEY_BASE + n)));
        }
    }

    fn release(self, renderer: &mut dyn RenderBackend) {
        self.parts.iter().for_each(|part| renderer.destroy_mesh(part.handle));
        self.textures.into_iter().for_each(|texture| renderer.destroy_texture(texture));
    }
}

#[derive(Default)]
enum State {
    /// Not looked for yet.
    #[default]
    Unloaded,
    /// The car has no rider files, or they did not load: nothing is drawn.
    Absent,
    Ready(Box<RiderRig>),
}

/// The rider of the player's bike, if the bike has one. Loaded the first frame the bike is drawn.
#[derive(Default)]
pub struct Rider {
    state: State,
}

impl Rider {
    /// One physics step: the bike's state (`None` for a car), `dt` seconds.
    pub fn step(&mut self, sample: Option<RiderSample>, dt: f32) {
        if let (State::Ready(rig), Some(sample)) = (&mut self.state, sample) {
            rig.step(&sample, dt);
        }
    }

    /// Once per frame: load the rider the first time a bike is drawn, then pose, skin and upload it for the bike's pose
    /// `pose` (the front wheel's steering turns the grips).
    pub fn maintain(
        &mut self,
        renderer: &mut dyn RenderBackend,
        dir: &GameDir,
        model: &CarModel,
        (pose, bike): (&CarPose, bool),
    ) {
        if matches!(self.state, State::Unloaded) {
            if !bike {
                return;
            }
            self.state = match RiderRig::load(renderer, dir, model) {
                Ok(Some(rig)) => State::Ready(Box::new(rig)),
                Ok(None) => {
                    log::info!("rider: {} has no rider files (attach.json); it is drawn without one", model.name);
                    State::Absent
                }
                Err(error) => {
                    log::warn!("rider: {} has no rider: {error:#}", model.name);
                    State::Absent
                }
            };
        }
        if let State::Ready(rig) = &mut self.state {
            rig.update(renderer, pose.wheels[0].steer);
        }
    }

    /// Append the rider's instances for the bike at `pose`.
    pub fn instances(&self, pose: &CarPose, out: &mut Vec<Instance>) {
        if let State::Ready(rig) = &self.state {
            rig.instances(pose.transform(), out);
        }
    }

    pub fn release(self, renderer: &mut dyn RenderBackend) {
        if let State::Ready(rig) = self.state {
            rig.release(renderer);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_skeleton::Influences;

    #[test]
    fn the_rider_files_are_looked_for_in_the_cars_folder_first() {
        assert_eq!(folders("KTM450"), ["CARS/KTM450/RIDER".to_owned(), "RIDER".to_owned()]);
    }

    #[test]
    fn the_graph_gets_its_parameters_by_name_in_its_own_order() {
        let values = Values { lean: -0.5, speed_kmh: 90.0, tuck: 0.25, wheelie: 0.75, stoppie: 0.125 };
        let names: Vec<String> =
            ["speed_kmh", "lean", "mystery", "stoppie", "tuck", "wheelie"].iter().map(|s| (*s).to_owned()).collect();
        assert_eq!(param_values(&names, &values), vec![90.0, -0.5, 0.0, 0.125, 0.25, 0.75]);
        assert!(param_values(&[], &values).is_empty());
    }

    #[test]
    fn skinned_positions_become_white_vertices_with_the_meshs_texture_coordinates() {
        let source =
            |uv: [f32; 2]| SkinVertex { position: Vec3::ZERO, normal: Vec3::Z, uv, influences: Influences::default() };
        let sources = [source([0.25, 0.5]), source([1.0, 0.0])];
        let positions = [Vec3::new(1.0, 2.0, 3.0), Vec3::new(4.0, 5.0, 6.0)];
        let normals = [Vec3::X, Vec3::Y];
        let mut out = vec![Vertex { position: [9.0; 3], normal: [9.0; 3], color_bgra: [0; 4], uv: [9.0; 2] }; 5];
        fill(&mut out, &sources, &positions, &normals);
        assert_eq!(out.len(), 2, "the old contents are replaced");
        assert_eq!(out[0].position, [1.0, 2.0, 3.0]);
        assert_eq!(out[1].normal, [0.0, 1.0, 0.0]);
        assert_eq!(out[0].uv, [0.25, 0.5]);
        assert_eq!(out[1].color_bgra, [255; 4]);
    }

    #[test]
    fn a_rider_that_is_not_loaded_is_not_stepped_or_drawn() {
        let mut rider = Rider::default();
        assert!(matches!(rider.state, State::Unloaded));
        rider.step(Some(RiderSample::default()), 0.016);
        let pose = CarPose { position: Vec3::ZERO, rotation: glam::Quat::IDENTITY, wheels: [Default::default(); 4] };
        let mut out = Vec::new();
        rider.instances(&pose, &mut out);
        assert!(out.is_empty());
    }
}
