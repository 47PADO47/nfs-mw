//! The characters of a NIS scene: the actors of the arrest scenes. Each record is one actor, a skinned model with a
//! skeleton and a clip. The model is split into one part per bone; each frame every part's vertices are skinned with
//! their blend weights (`skin.rs`) and drawn placed by the record's local matrix in the scene, the scene's anchor and a small lift. Each character keeps
//! its feet on the ground under its pelvis (`keep_on_ground`), as the original's keep-on-ground does. The rules are in
//! `docs/specs/nis-player-2.md`, section 9; the records in `docs/specs/nis-scenes.md`, section 4; the skin in
//! `docs/formats/models.md`, "Skinned vertices and bones".

use std::collections::HashMap;

use blackbox_anim::{Bank, Clip, Object, Pose, Skeleton};
use blackbox_gfx::{BlendMode, Instance, MeshHandle, RenderBackend, Shading, TextureHandle};
use blackbox_hash::bstring_hash;
use blackbox_nis::Character;
use blackbox_scene::{blend_mode, upload_rigid_part, upload_texture};
use blackbox_solid::{RigidPart, Solid, rigid_split};
use blackbox_tpk::TexturePack;
use glam::{Mat4, Vec3};
use nfsmw_data::nis::NisBundle;

use super::skin::skin_part;
use crate::scenes::world::keys;
use crate::scenes::world::scene_space::Anchor;

/// How far every character stands above its ground (or its placement, before a ground is found), in metres
/// (`nis-player-2.md`, section 9.3) [decomp].
const LIFT: f32 = 0.05;
/// The pelvis bone, whose ground the whole character stands on (`nis-player-2.md`, section 9.3) [decomp].
const PELVIS: usize = 1;
/// How far above the pelvis the ground under it is looked for, in metres (`nis-player-2.md`, section 9.3) [decomp].
const PELVIS_RAISE: f32 = 2.0;
/// The character clips run at half the scene's time (`nis-player-2.md`, section 9.1, step 3).
const CLIP_TIME_SCALE: f32 = 0.5;
/// The play-flag bit that makes a clip repeat until the scene ends (`nis-player-2.md`, section 9.1, step 4).
const LOOP_FLAG: u32 = 0x20;

/// Textures by name hash, each with how it blends.
pub type Materials = HashMap<u32, (TextureHandle, BlendMode)>;

/// The characters of one scene: their models and actors, and the GPU resources once they are uploaded.
#[derive(Default)]
pub struct Characters {
    models: Vec<Model>,
    actors: Vec<Actor>,
    /// The bundle's texture packs, until they are uploaded.
    packs: Vec<TexturePack>,
    /// Whether the models have been uploaded. They are uploaded once per scene.
    uploaded: bool,
    /// The textures the scene uploaded, freed with its meshes.
    textures: Vec<TextureHandle>,
}

/// One skinned solid that the characters use, split into its rigid parts.
struct Model {
    solid: Solid,
    parts: Vec<RigidPart>,
}

/// One character: which model it wears, its skeleton and clip, and where it stands.
struct Actor {
    /// Index into [`Characters::models`].
    model: usize,
    skeleton: Skeleton,
    clip: Clip,
    /// The record's local matrix: where the character stands in scene space.
    placement: Mat4,
    play_speed: f32,
    play_delay: f32,
    looping: bool,
    /// The ground the character stands on, render-space height, smoothed frame to frame; `None` until one is found.
    ground: Option<f32>,
    /// The GPU mesh of each part of the model, the actor's own (each actor is posed on its own) once uploaded. `None`
    /// for a part that was not uploaded.
    meshes: Vec<Option<MeshHandle>>,
    /// The backend could not take the skinned vertices: the parts are drawn rigidly, each with its bone's matrix.
    rigid: bool,
}

impl Characters {
    /// The characters of a bundle's scene: one actor per record whose clip, skeleton and model are all in the bundle.
    /// A record that does not resolve is logged and left out.
    pub fn from_bundle(bundle: &NisBundle) -> Self {
        let mut characters = Self::default();
        for record in &bundle.scene.characters {
            if let Err(reason) = characters.add(record, bundle) {
                log::warn!("NIS {}: a character is not drawn: {reason}", bundle.name);
            }
        }
        if !characters.actors.is_empty() {
            characters.packs = bundle.textures.clone();
        }
        characters
    }

    /// Adds the actor of one record. Fails, naming the hash that did not resolve, when the clip, the skeleton or the
    /// model is missing from the bundle.
    fn add(&mut self, record: &Character, bundle: &NisBundle) -> Result<(), String> {
        let clip = find_clip(&bundle.anim.banks, record.skeletal_anim_hash)
            .ok_or_else(|| format!("no clip with hash {:#010x}", record.skeletal_anim_hash))?;
        let skeleton = skeleton_index(&bundle.anim.skeletons, record.skeleton_name_hash, Some(clip.checksum))
            .map(|index| bundle.anim.skeletons[index].clone())
            .ok_or_else(|| format!("no skeleton with hash {:#010x}", record.skeleton_name_hash))?;
        let solid = solid_index(&bundle.solids, record.lod_name_hashes)
            .ok_or_else(|| format!("no model for the LOD hashes {:x?}", record.lod_name_hashes))?;
        let model = self.model_of(&bundle.solids[solid]);
        self.actors.push(Actor {
            model,
            skeleton,
            clip,
            placement: record.local_matrix,
            play_speed: record.play_speed,
            play_delay: record.play_delay,
            looping: record.play_flags & LOOP_FLAG != 0,
            ground: None,
            meshes: Vec::new(),
            rigid: false,
        });
        Ok(())
    }

    /// The index of the model of `solid`, adding it on first use. One model serves every actor that wears the solid.
    fn model_of(&mut self, solid: &Solid) -> usize {
        if let Some(index) = self.models.iter().position(|model| model.solid.name_hash == solid.name_hash) {
            return index;
        }
        let mut parts = rigid_split(solid);
        // The skinned shader ignores the vertex colour, and the character meshes store it as 0; the lit material
        // multiplies by it, so it is set to white.
        for vertex in parts.iter_mut().flat_map(|part| part.vertices.iter_mut()) {
            vertex.color_bgra = [255; 4];
        }
        self.models.push(Model { solid: solid.clone(), parts });
        self.models.len() - 1
    }

    /// Uploads the models' parts and the scene's textures, once. A texture of the bundle comes first, then `shared`,
    /// the world's own textures. Does nothing for a scene without characters, or once the scene is uploaded.
    pub fn prepare(&mut self, renderer: &mut dyn RenderBackend, shared: &Materials) {
        if self.uploaded || self.models.is_empty() {
            return;
        }
        self.uploaded = true;
        let mut own = Materials::new();
        for texture in self.packs.iter().flat_map(|pack| &pack.textures) {
            let Some(handle) = upload_texture(renderer, texture) else { continue };
            own.insert(texture.name_hash, (handle, blend_mode(Some(texture))));
            self.textures.push(handle);
        }
        self.packs.clear();
        let lookup = |hash: u32| own.get(&hash).or_else(|| shared.get(&hash)).copied();
        for actor in &mut self.actors {
            let Some(model) = self.models.get(actor.model) else { continue };
            actor.meshes = model
                .parts
                .iter()
                .map(|part| upload_rigid_part(renderer, &model.solid, part, &lookup, Shading::Lit))
                .collect();
        }
    }

    /// Finds the ground under each actor's pelvis at scene time `scene_time`, with the scene standing at `anchor`.
    /// `ground(at)` gives the height of the first ground going down from `at` (render space). The pelvis is looked under
    /// from [`PELVIS_RAISE`] above it, and the height found is smoothed half and half with the last one. An actor with no
    /// ground found keeps its last one. The world calls it every frame while the scene plays.
    pub fn keep_on_ground(&mut self, scene_time: f32, anchor: &Anchor, mut ground: impl FnMut(Vec3) -> Option<f32>) {
        let to_render = scene_to_render(anchor);
        for actor in &mut self.actors {
            let placed = to_render * actor.placement;
            let Some(pelvis) = actor.bone_position(scene_time, PELVIS) else { continue };
            let raise = ground_raise(actor.ground, placed.w_axis.z);
            let at = placed.transform_point3(pelvis) + Vec3::new(0.0, 0.0, raise + PELVIS_RAISE);
            let Some(height) = ground(at) else { continue };
            actor.ground = Some(smoothed_ground(actor.ground, height));
        }
    }

    /// Skins every actor's parts for scene time `scene_time` and gives the vertices to their meshes (`skin.rs`). An
    /// actor whose mesh the backend cannot update falls back to the rigid draw for the rest of the scene. The world calls
    /// it every frame after [`Characters::prepare`].
    pub fn skin(&mut self, renderer: &mut dyn RenderBackend, scene_time: f32) {
        let (mut posed, mut drawn) = (Vec::new(), Vec::new());
        for actor in self.actors.iter_mut().filter(|actor| !actor.rigid) {
            let Some(model) = self.models.get(actor.model) else { continue };
            let Some(skin) = model.solid.skin.as_deref() else { continue };
            let skinning = actor.skinning(scene_time);
            for (part, mesh) in model.parts.iter().zip(&actor.meshes) {
                let Some(mesh) = *mesh else { continue };
                skin_part(part, skin, &skinning, &mut posed);
                drawn.clear();
                drawn.extend(posed.iter().map(|v| blackbox_gfx::Vertex {
                    position: v.position,
                    normal: v.normal,
                    color_bgra: v.color_bgra,
                    uv: v.uv,
                }));
                if !renderer.update_mesh_vertices(mesh, &drawn) {
                    actor.rigid = true;
                    break;
                }
            }
        }
    }

    /// Adds one instance per uploaded part of every actor, at scene time `scene_time` with the scene standing at
    /// `anchor`. Nothing is added before [`Characters::prepare`].
    pub fn instances(&self, scene_time: f32, anchor: &Anchor, out: &mut Vec<Instance>) {
        let to_render = scene_to_render(anchor);
        for (actor_index, actor) in self.actors.iter().enumerate() {
            let Some(model) = self.models.get(actor.model) else { continue };
            let skinning = actor.skinning(scene_time);
            let raise = ground_raise(actor.ground, (to_render * actor.placement).w_axis.z);
            let lifted = Mat4::from_translation(Vec3::new(0.0, 0.0, raise)) * to_render;
            for (part_index, (part, mesh)) in model.parts.iter().zip(&actor.meshes).enumerate() {
                let (Some(mesh), Some(bone)) = (*mesh, skinning.get(usize::from(part.bone))) else { continue };
                // Skinned parts already carry the pose in their vertices; the rigid fallback moves each with its bone.
                let pose = if actor.rigid { *bone } else { Mat4::IDENTITY };
                let transform = part_transform(lifted, actor.placement, pose);
                out.push(Instance::keyed(mesh, transform, keys::character(actor_index, part_index)));
            }
        }
    }

    /// Frees the meshes and textures the scene uploaded.
    pub fn release(self, renderer: &mut dyn RenderBackend) {
        for actor in self.actors {
            for mesh in actor.meshes.into_iter().flatten() {
                renderer.destroy_mesh(mesh);
            }
        }
        for texture in self.textures {
            renderer.destroy_texture(texture);
        }
    }
}

impl Actor {
    /// The clip's pose at scene time `scene_time`, over the bind pose.
    fn pose(&self, scene_time: f32) -> Pose {
        let secs = clip_seconds(scene_time, self.play_speed, self.play_delay, self.looping, self.clip.duration_secs());
        self.clip.sample_pose(secs, &self.skeleton.bind_pose())
    }

    /// The skinning matrix of every bone at scene time `scene_time`.
    fn skinning(&self, scene_time: f32) -> Vec<Mat4> {
        self.skeleton.skinning(&self.pose(scene_time))
    }

    /// Where bone `bone` is at scene time `scene_time`, in the actor's own frame (before its placement). `None` for a
    /// rig without that bone.
    fn bone_position(&self, scene_time: f32, bone: usize) -> Option<Vec3> {
        self.skeleton.world(&self.pose(scene_time)).get(bone).map(|matrix| matrix.w_axis.truncate())
    }
}

/// The ground an actor stands on after a frame that found `found` under it: the first ground found, then half the new
/// height and half the last (`nis-player-2.md`, section 9.3) [decomp].
pub fn smoothed_ground(last: Option<f32>, found: f32) -> f32 {
    last.map_or(found, |last| found * 0.5 + last * 0.5)
}

/// How far an actor whose placement puts its root at render height `root_z` is raised to stand on `ground`: the
/// difference, or nothing before a ground is found. [`LIFT`] comes on top in [`part_transform`].
pub fn ground_raise(ground: Option<f32>, root_z: f32) -> f32 {
    ground.map_or(0.0, |ground| ground - root_z)
}

/// The scene's space in render space: turned about the up axis by the anchor, then moved to its origin. Points get the
/// same placement from [`Anchor::to_render`].
pub fn scene_to_render(anchor: &Anchor) -> Mat4 {
    Mat4::from_translation(anchor.origin) * Mat4::from_quat(anchor.rotation())
}

/// The transform of one part of a character: the scene's placement, the actor's placement in the scene, the lift, then
/// the bone's skinning matrix. The part's vertices sit at the bind pose in the rig's frame, so the skinning matrix moves
/// them to the posed bone first.
pub fn part_transform(to_render: Mat4, placement: Mat4, skinning: Mat4) -> Mat4 {
    to_render * placement * Mat4::from_translation(Vec3::new(0.0, 0.0, LIFT)) * skinning
}

/// The clip's time in seconds at scene time `scene_time`: the time after the record's delay, times its play speed and
/// the half-rate scale. A looping clip wraps at its length. Any other clip holds its last frame, as [`Clip`] clamps it.
pub fn clip_seconds(scene_time: f32, play_speed: f32, play_delay: f32, looping: bool, duration: f32) -> f32 {
    let secs = (scene_time - play_delay).max(0.0) * play_speed * CLIP_TIME_SCALE;
    match looping && duration > 0.0 {
        true => secs.rem_euclid(duration),
        false => secs,
    }
}

/// The skeleton of a record: the one whose name hashes to `name_hash`, else the first with `checksum`. The record stores
/// no checksum of its own, so the checksum is the clip's target. `None` when neither matches.
pub fn skeleton_index(skeletons: &[Skeleton], name_hash: u32, checksum: Option<u16>) -> Option<usize> {
    let by_name = skeletons.iter().position(|skeleton| bstring_hash(&skeleton.name) == name_hash);
    by_name.or_else(|| checksum.and_then(|checksum| skeletons.iter().position(|s| s.checksum == checksum)))
}

/// The model of a record: the solid that the first non-zero LOD hash names, LOD 0 first. `None` when no LOD names a
/// solid of the bundle.
pub fn solid_index(solids: &[Solid], lod_name_hashes: [u32; 4]) -> Option<usize> {
    lod_name_hashes
        .iter()
        .filter(|&&hash| hash != 0)
        .find_map(|&hash| solids.iter().position(|solid| solid.name_hash == hash))
}

/// The clip with name hash `hash`, read from the first bank that holds it. A clip that does not read is logged and
/// `None`.
fn find_clip(banks: &[(Object, Bank)], hash: u32) -> Option<Clip> {
    banks.iter().find_map(|(object, bank)| {
        let animation = bank.iter().find(|animation| bstring_hash(&animation.name) == hash)?;
        match Clip::read(object, animation) {
            Ok(clip) => Some(clip),
            Err(error) => {
                log::warn!("NIS clip {}: {error}", animation.name);
                None
            }
        }
    })
}

#[cfg(test)]
#[path = "characters_tests.rs"]
mod tests;
