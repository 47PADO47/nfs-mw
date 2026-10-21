//! `view-anim`: one EAGL skeleton of a bundle drawn as sticks, a thin box per bone from its parent's joint to its own
//! and a small cube at each joint, animated by one of the bundle's clips. The orbit camera turns about the rig (drag to
//! orbit, scroll to zoom, Esc quits).

mod bones;

use anyhow::{Context, Result, anyhow, bail};
use blackbox_anim::{Clip, Pose, Skeleton};
use blackbox_gfx::{BlendMode, DrawRange, FrameParams, Instance, MeshDesc, MeshHandle, RenderBackend, Shading};
use blackbox_scene::Aabb;
use game_install::GameDir;
use glam::{Mat4, Vec3};
use nfsmw_data::anim::AnimObjects;

use crate::input::ActionState;
use crate::viewer::Scene;
use crate::viewer::camera::{Camera, CameraCut, OrbitCamera};

/// The direction the light travels in, as the car viewer's.
const LIGHT_DIR: Vec3 = Vec3::new(-0.4, -0.3, -1.0);
/// The yaw the rig starts at, as the car viewer's default.
const START_YAW: f32 = 35.0;
/// The pitch the camera starts at: a little above the rig.
const START_PITCH: f32 = 0.35;
/// Stick thickness and joint size as shares of the rig's extent (its bounding box diagonal).
const THICKNESS: f32 = 0.012;
const JOINT_SIZE: f32 = 0.03;
/// The smallest extent the sizes are taken from, in case the rig has one bone or sits on a point.
const MIN_EXTENT: f32 = 1.0;
const BONE_COLOR: [u8; 4] = [0x40, 0xA0, 0xE0, 0xFF];
const JOINT_COLOR: [u8; 4] = [0xD0, 0xD0, 0xD0, 0xFF];

/// The two meshes the rig is drawn with, both unit cubes in their own colour.
struct Meshes {
    bone: MeshHandle,
    joint: MeshHandle,
}

pub struct AnimViewScene {
    skeleton: Skeleton,
    /// The parent of every bone, from the skeleton, kept for the frame's boxes.
    parents: Vec<Option<usize>>,
    bind: Pose,
    /// The clip that drives the rig, or `None` when the bundle has no clips (the rig then stays in its bind pose).
    clip: Option<Clip>,
    /// Seconds into the clip. Starts at `--time` and loops over the clip's length.
    clock: f32,
    camera: OrbitCamera,
    cut: CameraCut,
    /// Stick thickness and joint size, in the rig's units.
    thickness: f32,
    joint_size: f32,
    meshes: Option<Meshes>,
    instances: Vec<Instance>,
    title: String,
}

impl AnimViewScene {
    /// Opens the rig `rig` names (or the bundle's first) and the clip `clip` names. Without a name the clip is the
    /// first one that drives that rig (by checksum), else the bundle's first clip. `time` is the clip time to start at.
    pub fn open(dir: &GameDir, path: &str, rig: Option<&str>, clip: Option<&str>, time: f32) -> Result<Self> {
        let objects = AnimObjects::load(dir, path)?;
        let skeleton = pick_skeleton(&objects.skeletons, rig)?.clone();
        if skeleton.bones.is_empty() {
            bail!("the rig {} has no bones to draw", skeleton.name);
        }
        let clip = pick_clip(read_clips(&objects)?, &skeleton, clip)?;
        let bind = skeleton.bind_pose();
        let parents: Vec<Option<usize>> = skeleton.bones.iter().map(|bone| bone.parent).collect();
        let joints = bones::joints(&skeleton.world(&bind));
        let bounds = joint_bounds(&joints);
        let extent = (bounds.max - bounds.min).length().max(MIN_EXTENT);
        // The orbit sits as the car viewer's does: a distance of 2.2 radii from the centre of the bounds.
        let camera = OrbitCamera {
            target: bounds.center(),
            distance: extent * 1.1,
            yaw: START_YAW.to_radians(),
            pitch: START_PITCH,
        };
        let clip_name = clip.as_ref().map_or("bind pose", |clip| clip.name.as_str());
        let title = format!("nfsmw — {} {clip_name}", skeleton.name);
        Ok(Self {
            skeleton,
            parents,
            bind,
            clip,
            clock: time,
            camera,
            cut: CameraCut::default(),
            thickness: extent * THICKNESS,
            joint_size: extent * JOINT_SIZE,
            meshes: None,
            instances: Vec::new(),
            title,
        })
    }

    /// The rig's pose `clock` seconds in: the clip's pose, looped over its length, or the bind pose without a clip.
    fn pose(&self) -> Pose {
        let Some(clip) = &self.clip else { return self.bind.clone() };
        clip.sample_pose(bones::looped(self.clock, clip.duration_secs()), &self.bind)
    }

    /// Lists this frame's boxes: every bone's stick, then a cube at every joint. Each mesh's instances are adjacent.
    fn rebuild(&mut self) {
        let Some(meshes) = &self.meshes else { return };
        let joints = bones::joints(&self.skeleton.world(&self.pose()));
        self.instances.clear();
        let sticks = bones::sticks(&self.parents, &joints, self.thickness);
        for transform in sticks.into_iter().flatten() {
            self.instances.push(Instance::new(meshes.bone, transform));
        }
        for joint in joints {
            let transform = Mat4::from_translation(joint) * Mat4::from_scale(Vec3::splat(self.joint_size));
            self.instances.push(Instance::new(meshes.joint, transform));
        }
    }

    /// Frees the two meshes. Calling it again does nothing.
    fn release(&mut self, renderer: &mut dyn RenderBackend) {
        if let Some(meshes) = self.meshes.take() {
            renderer.destroy_mesh(meshes.bone);
            renderer.destroy_mesh(meshes.joint);
        }
        self.instances.clear();
    }
}

/// The skeleton called `rig`, or the bundle's first one. Names the ones there are when `rig` is not among them.
fn pick_skeleton<'a>(skeletons: &'a [Skeleton], rig: Option<&str>) -> Result<&'a Skeleton> {
    let Some(rig) = rig else {
        return skeletons.first().ok_or_else(|| anyhow!("the bundle has no skeletons"));
    };
    if let Some(found) = skeletons.iter().find(|skeleton| skeleton.name == rig) {
        return Ok(found);
    }
    let names: Vec<&str> = skeletons.iter().map(|skeleton| skeleton.name.as_str()).collect();
    bail!("no skeleton {rig:?} in this bundle (it has: {})", names.join(", "))
}

/// Every clip of the bundle's banks, in bank order.
fn read_clips(objects: &AnimObjects) -> Result<Vec<Clip>> {
    let mut clips = Vec::new();
    for (object, bank) in &objects.banks {
        for animation in bank.iter() {
            let clip = Clip::read(object, animation).with_context(|| format!("clip {}", animation.name))?;
            clips.push(clip);
        }
    }
    Ok(clips)
}

/// The clip called `name`, or else the first clip that drives `skeleton` (by checksum), or else the first clip. `None`
/// when the bundle has no clips at all.
fn pick_clip(mut clips: Vec<Clip>, skeleton: &Skeleton, name: Option<&str>) -> Result<Option<Clip>> {
    if let Some(name) = name {
        let Some(index) = clips.iter().position(|clip| clip.name == name) else {
            bail!("no clip {name:?} in this bundle (dump-anim lists them)");
        };
        return Ok(Some(clips.swap_remove(index)));
    }
    let index = clips.iter().position(|clip| clip.checksum == skeleton.checksum).or((!clips.is_empty()).then_some(0));
    Ok(index.map(|index| clips.swap_remove(index)))
}

/// The box that holds every joint. The caller has checked that there is at least one.
fn joint_bounds(joints: &[Vec3]) -> Aabb {
    let mut bounds = Aabb::EMPTY;
    for &joint in joints {
        bounds.extend(joint);
    }
    bounds
}

impl Scene for AnimViewScene {
    fn close(&mut self, renderer: &mut dyn RenderBackend) {
        self.release(renderer);
    }

    fn title(&self) -> String {
        self.title.clone()
    }

    fn init(&mut self, renderer: &mut dyn RenderBackend) -> Result<()> {
        let bone = upload(renderer, "anim stick", BONE_COLOR);
        let joint = upload(renderer, "anim joint", JOINT_COLOR);
        self.meshes = Some(Meshes { bone, joint });
        self.rebuild();
        Ok(())
    }

    fn update(&mut self, _renderer: &mut dyn RenderBackend, input: &ActionState, dt: f32) {
        self.camera.update(input);
        self.clock += dt;
        self.rebuild();
    }

    fn frame(&mut self, aspect: f32) -> (FrameParams, &[Instance]) {
        let lens = self.camera.lens(aspect);
        let params = FrameParams {
            view: lens.view,
            projection: lens.projection,
            camera_position: lens.position,
            light_dir: LIGHT_DIR,
            clear_color: [0.18, 0.2, 0.24],
            fog: None,
            scattering: None,
            surface_light: None,
            camera_cut: self.cut.take(),
            shadow_tuning: None,
        };
        (params, &self.instances)
    }
}

/// A unit cube in one colour, uploaded as a mesh that the rig's boxes scale and place.
fn upload(renderer: &mut dyn RenderBackend, label: &str, color: [u8; 4]) -> MeshHandle {
    let (vertices, indices) = bones::cube(color);
    renderer.create_mesh(&MeshDesc {
        secondary_uvs: None,
        tangents: None,
        label,
        vertices: &vertices,
        indices: &indices,
        draws: vec![DrawRange {
            first_index: 0,
            index_count: indices.len() as u32,
            base_vertex: 0,
            texture: None,
            blend: BlendMode::Opaque,
            shading: Shading::Lit,
        }],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rig(name: &str) -> Skeleton {
        Skeleton { name: name.to_string(), checksum: 7, bones: Vec::new() }
    }

    #[test]
    fn the_first_rig_is_the_default_and_a_named_one_is_found_by_its_exact_name() {
        let rigs = [rig("Arrest"), rig("Cop")];
        assert_eq!(pick_skeleton(&rigs, None).unwrap().name, "Arrest");
        assert_eq!(pick_skeleton(&rigs, Some("Cop")).unwrap().name, "Cop");
        let err = pick_skeleton(&rigs, Some("Civ")).unwrap_err().to_string();
        assert!(err.contains("Arrest, Cop"), "the error names the rigs: {err}");
        assert!(pick_skeleton(&[], None).is_err());
    }

    #[test]
    fn the_bounds_hold_every_joint() {
        let joints = [Vec3::new(-1.0, 0.0, 2.0), Vec3::new(3.0, -2.0, 0.5)];
        let bounds = joint_bounds(&joints);
        assert_eq!(bounds.min, Vec3::new(-1.0, -2.0, 0.5));
        assert_eq!(bounds.max, Vec3::new(3.0, 0.0, 2.0));
    }
}
