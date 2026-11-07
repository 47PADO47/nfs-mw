//! A rider on a two-wheeler: a skinned, animated character drawn on the player's bike.
//!
//! A mod gives a bike a rider with files (`docs/formats/ride6-rider.md`; a mod's `files/CARS/<ID>/RIDER/` or `files/RIDER/`):
//! `rider.bbsk` (the skeleton, the animations and the skinned meshes), `graph.json` (the blend tree), `attach.json` (the seat,
//! the grips, the pegs, the limbs to solve and the numbers that drive the animation), `parts.json` (which picture each mesh
//! wears) and `textures/*.png`. A bike without `attach.json` has no rider; the game says so once and goes on. A bike's own
//! `files/CARS/<ID>/RIDER/contacts.json` puts the seat, the hubs and the grip and peg points of that bike over the shared
//! `attach.json` ([`contacts`]), so the rider reaches each bike's own controls.
//!
//! Each physics step feeds the rider the bike's state ([`Rider::step`]: the smoothed inputs of the graph and the clock). Each
//! frame the rider is posed ([`pose`]: the graph, the body's shift, the hands on the grips and the feet on the pegs), skinned
//! on the CPU and its vertices written to the meshes the renderer holds ([`Rider::maintain`]), and drawn at the bike's
//! transform ([`Rider::instances`]). Nothing here names a game: the files say what the bones are called.

mod attach;
mod contacts;
mod fall;
mod fall_anim;
mod fall_file;
mod fall_space;
#[cfg(test)]
mod fit_tests;
mod foot_out;
mod gear;
mod graph_params;
mod lean_pose;
mod params;
mod parts;
mod pose;
#[cfg(test)]
mod probe_tests;
mod reach;
mod reverse;
mod script;
mod seat;
#[cfg(test)]
mod smooth_tests;
mod style;
mod view;
mod worn;

use std::sync::Arc;

use anyhow::{Context, Result};
use blackbox_gfx::{DrawRange, Instance, MeshDesc, MeshHandle, RenderBackend, TextureHandle, Vertex};
use blackbox_skeleton::{Graph, Pack, SkinVertex, skin_matrices, skin_vertices};
use game_install::GameDir;
use glam::{Mat4, Vec3};
use nfsmw_data::car::CarModel;
use nfsmw_data::car::mods::bike_camera::BikeCamera;

use self::attach::Attach;
pub use self::fall::FallStep;
pub use self::graph_params::param_values;
pub use self::params::RiderSample;
use self::params::Smoothed;
use self::pose::{Frame, Resolved, Steering};
use super::CarPose;
use crate::garage::rider_gear::{self, Choice};
use crate::scenes::world::keys;

/// The first part number a rider's meshes take in the instance keys of the player's car (the car's own placements come
/// before it).
const KEY_BASE: usize = 0x4000;

/// Where the rider files of the car `car` are looked for, the first folder with an `attach.json` winning: the car's own, then
/// the one every bike of the mod shares.
pub fn folders(car: &str) -> [String; 2] {
    [format!("CARS/{car}/RIDER"), "RIDER".to_owned()]
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
    /// The rider coming off the bike in a fall (`fall.rs`).
    fall: fall::Fall,
    /// Where his eyes are and which meshes the first-person camera leaves out (`view.rs`).
    view: view::EyeView,
    /// The folder the rig was read from, the entries of its `parts.json` (his own gear) and how many meshes its pack had: the
    /// gear he wears adds meshes after those (`gear.rs`).
    folder: String,
    stock: Vec<parts::Entry>,
    base_meshes: usize,
    /// The gear on him now, and the camera table whose helmet names say which meshes to leave out from inside.
    gear: Choice,
    cameras: Option<Arc<BikeCamera>>,
}

impl RiderRig {
    /// Read and upload the rider of `model`, in the gear `choice`. `Ok(None)` when the car has no rider files.
    fn load(renderer: &mut dyn RenderBackend, dir: &GameDir, model: &CarModel, choice: Choice) -> Result<Option<Self>> {
        let Some(folder) = folders(&model.name).into_iter().find(|f| dir.exists(&format!("{f}/attach.json"))) else {
            return Ok(None);
        };
        let read =
            |name: &str| dir.read(&format!("{folder}/{name}")).with_context(|| format!("reading {folder}/{name}"));
        let attach = Attach::parse(&read("attach.json")?)?;
        // The bike's own seat, grips and pegs (`CARS/<ID>/RIDER/contacts.json`) over the rig's.
        let (attach, kind) = contacts::lay_over(attach, dir, &model.name);
        let mut pack = Pack::parse(&read("rider.bbsk")?).context("rider.bbsk")?;
        // The blend tree of this bike's kind (`styles.json`), else the rig's own.
        let selection = style::select(dir, &folder, kind.as_deref());
        let graph: Graph = serde_json::from_slice(&read(&selection.graph)?).with_context(|| selection.graph.clone())?;
        graph
            .validate(&pack.clips, &pack.skeleton)
            .with_context(|| format!("{} does not fit rider.bbsk", selection.graph))?;
        log::info!("rider: {} plays {} ({})", model.name, selection.graph, kind.as_deref().unwrap_or("no kind"));
        let mut resolved = Resolved::new(&attach, &pack.skeleton).map_err(anyhow::Error::msg)?;
        resolved.seat = seat::Seat::new(&attach, &pack);
        let stock = parts::parse_entries(&read("parts.json")?)?;
        let corners = model.corners.as_ref().context("the car has no wheel mounts")?;
        let (front, rear) = (corners[0], corners[2]);
        let frame =
            Frame::new(&attach.rider_root_in_bike.transform(), attach.frame_offset(front.centre(), rear.centre()));

        let base_meshes = pack.meshes.len();
        if rider_gear::loaded().is_none() {
            rider_gear::scan(dir);
        }
        let catalogue = rider_gear::loaded();
        let wearing = catalogue.as_deref().map(|catalogue| (catalogue, &choice));
        let worn = worn::wear(renderer, dir, &folder, &mut pack, &stock, wearing);
        let cameras = nfsmw_data::car::mods::bike_camera::of(&model.name);
        let names: Vec<&str> = worn.parts.iter().map(|part| pack.meshes[part.mesh].name.as_str()).collect();
        let view = view::EyeView::new(cameras.as_ref().map(|set| &set.helmet), &pack.skeleton, &names);
        log::info!(
            "rider: {} meshes, {} pictures, {} bones from {folder}; {}; gear: {}",
            worn.parts.len(),
            worn.textures.len(),
            pack.skeleton.len(),
            match resolved.seat.is_some() {
                true => "sat on the bike's seat",
                false => "no seat surface for this bike: he sits at the socket",
            },
            match choice.is_stock() {
                true => "his own".to_owned(),
                false => gear::describe(&choice),
            }
        );
        let mut smoothed = Smoothed::default();
        smoothed.set_walk(selection.walk);
        smoothed.set_kind(selection.bike_type, selection.riding_style);
        let fall = fall::Fall::load(dir, &folder, &pack);
        Ok(Some(Self {
            pack,
            graph,
            attach,
            resolved,
            frame,
            steering: front.steering_axis(),
            smoothed,
            time: 0.0,
            parts: worn.parts,
            textures: worn.textures,
            matrices: Vec::new(),
            positions: Vec::new(),
            normals: Vec::new(),
            failed: false,
            fall,
            view,
            folder,
            stock,
            base_meshes,
            gear: choice,
            cameras,
        }))
    }

    /// Frees the meshes and pictures he is drawn with.
    fn drop_parts(&mut self, renderer: &mut dyn RenderBackend) {
        self.parts.drain(..).for_each(|part| renderer.destroy_mesh(part.handle));
        self.textures.drain(..).for_each(|texture| renderer.destroy_texture(texture));
    }

    /// Dresses him in the gear `choice` instead: his meshes and pictures are made again, the animation goes on.
    fn dress_again(&mut self, renderer: &mut dyn RenderBackend, dir: &GameDir, choice: Choice) {
        self.drop_parts(renderer);
        self.pack.meshes.truncate(self.base_meshes);
        let catalogue = rider_gear::loaded();
        let wearing = catalogue.as_deref().map(|catalogue| (catalogue, &choice));
        let worn = worn::wear(renderer, dir, &self.folder, &mut self.pack, &self.stock, wearing);
        self.parts = worn.parts;
        self.textures = worn.textures;
        let names: Vec<&str> = self.parts.iter().map(|part| self.pack.meshes[part.mesh].name.as_str()).collect();
        let view = view::EyeView::new(self.cameras.as_ref().map(|set| &set.helmet), &self.pack.skeleton, &names);
        self.view = view;
        self.gear = choice;
    }

    /// One physics step of `dt` seconds with the bike in state `sample`.
    fn step(&mut self, sample: &RiderSample, dt: f32) {
        self.smoothed.step(sample, &self.attach.inertia, dt);
        if dt.is_finite() && dt > 0.0 {
            self.time += dt.min(0.1);
        }
    }

    /// Pose the rider with the handlebar at `steer` (rad, positive left), skin it and write the vertices to the renderer.
    fn update(&mut self, renderer: &mut dyn RenderBackend, steer: f32, world: Mat4) {
        let steering = self.steering.map(|(axis, point)| Steering { axis, point, angle: steer });
        let targets = pose::rider_targets(&self.attach, &self.frame, steering);
        let body = pose::body(&self.frame, self.smoothed.shift(), self.smoothed.rotation());
        let params = param_values(&self.graph.param_names, &self.smoothed.values());
        let skeleton = &self.pack.skeleton;
        let release = self.smoothed.values().foot_release;
        let goals = pose::Goals { targets: &targets, body, frame: &self.frame, release };
        let animated =
            pose::animate(skeleton, &self.pack.clips, &self.graph, (&params, self.time), &self.resolved, &goals);
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
        // Off the bike in a fall, the pose is the ragdoll's; on it, the riding pose is remembered for the fall to begin from.
        let component = match self.fall.pose(skeleton, &component, world * self.frame.to_model) {
            Some(posed) => posed,
            None => {
                self.fall.remember(&component);
                component
            }
        };
        self.view.track(&component, self.frame.to_model, !self.fall.active());
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
    fn instances(&self, world: Mat4, first_person: bool, out: &mut Vec<Instance>) {
        let transform = match self.fall.active() {
            true => self.fall.instance(),
            false => world * self.frame.to_model,
        };
        for (n, part) in self.parts.iter().enumerate() {
            // From inside his helmet the rider's helmet and visor are not in the picture.
            if first_person && self.view.hidden.contains(&n) {
                continue;
            }
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
    /// The model the rider was loaded for: another model (a bike chosen in the garage) has other grips, pegs and seat, so the
    /// rider is loaded again for it.
    loaded_for: String,
    /// The rider is not drawn: the arrest scene plays the player's bike on its own, and the rider is not part of it.
    hidden: bool,
    /// The camera is inside his helmet: the helmet and the visor are not drawn (`view.rs`).
    first_person: bool,
    /// Where he looks from straight ahead, turned by the player's look stick (`viewer/camera/head_look.rs`).
    head_look: crate::viewer::camera::head_look::HeadLook,
    /// The rider of the garage's backdrop: he wears the gear on show there before it is kept, not only the kept gear.
    previewing: bool,
    /// The state of the gear (`garage/rider_gear`) as of the last frame: he is dressed again only when it moved.
    seen: u64,
}

impl Rider {
    /// A rider for a bike on show in the garage or the car viewer: he wears the gear being looked at (`rider_gear::set_preview`).
    pub fn previewing() -> Self {
        Self { previewing: true, ..Self::default() }
    }

    /// The gear he is to wear now.
    fn wanted(&self) -> Choice {
        match self.previewing {
            true => rider_gear::current(),
            false => rider_gear::committed(),
        }
    }

    /// One frame of the look stick (`stick`: x right, y up, -1 to 1): his head follows it and springs back when it is let go.
    pub fn look_around(&mut self, stick: [f32; 2], dt: f32) {
        self.head_look.update(stick, dt);
    }

    /// Where his head is turned to now.
    pub fn head_look(&self) -> crate::viewer::camera::head_look::HeadLook {
        self.head_look
    }

    /// One physics step: the bike's state (`None` for a car), `dt` seconds.
    pub fn step(&mut self, sample: Option<RiderSample>, dt: f32) {
        if let (State::Ready(rig), Some(sample)) = (&mut self.state, sample) {
            rig.step(&sample, dt);
        }
    }

    /// One physics step of the rider's fall (`fall.rs`): whether the bike let him go in this step, whether he is off it, and
    /// the ground the ragdoll falls on. `pose` is where the bike is. True when a bike lying still may be stood up now: the rider
    /// has got up (or there is no rider who falls).
    pub fn fall_step(&mut self, step: FallStep, pose: &CarPose, ground: &dyn blackbox_vehicle::Ground) -> bool {
        let State::Ready(rig) = &mut self.state else { return true };
        rig.fall_step(step, pose.transform(), ground)
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
        if self.loaded_for != model.name && !matches!(self.state, State::Unloaded) {
            if let State::Ready(rig) = std::mem::take(&mut self.state) {
                rig.release(renderer);
            }
            self.loaded_for.clear();
        }
        if matches!(self.state, State::Unloaded) {
            if !bike {
                return;
            }
            self.loaded_for.clone_from(&model.name);
            rider_gear::reload();
            self.seen = rider_gear::generation();
            self.state = match RiderRig::load(renderer, dir, model, self.wanted()) {
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
        let generation = rider_gear::generation();
        if self.seen != generation {
            self.seen = generation;
            let wanted = self.wanted();
            if let State::Ready(rig) = &mut self.state
                && rig.gear != wanted
            {
                rig.dress_again(renderer, dir, wanted);
            }
        }
        if let State::Ready(rig) = &mut self.state {
            rig.update(renderer, pose.wheels[0].steer, pose.transform());
        }
    }

    /// Hide the rider (the arrest scene) or show him again.
    pub fn set_hidden(&mut self, hidden: bool) {
        self.hidden = hidden;
    }

    /// Whether the camera is inside the rider's helmet, so his helmet and visor are left out of the picture.
    pub fn set_first_person(&mut self, on: bool) {
        self.first_person = on;
    }

    /// Where the rider's eyes are in the bike's model frame as of the last frame's pose: `None` with no rider, off the bike, or
    /// with no eye bone to follow.
    pub fn eye(&self) -> Option<glam::Vec3> {
        match &self.state {
            State::Ready(rig) => rig.view.eye(),
            _ => None,
        }
    }

    /// Append the rider's instances for the bike at `pose`.
    pub fn instances(&self, pose: &CarPose, out: &mut Vec<Instance>) {
        if self.hidden {
            return;
        }
        if let State::Ready(rig) = &self.state {
            rig.instances(pose.transform(), self.first_person, out);
        }
    }

    pub fn release(self, renderer: &mut dyn RenderBackend) {
        if let State::Ready(rig) = self.state {
            rig.release(renderer);
        }
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
