//! The driver as a figure that can be posed. Spec: `docs/formats/chameleon-driver-animation.md`.
//!
//! The character the car seats is skinned to a 46-bone skeleton and has an animation list of 28 clips. The game picks the clip
//! of the moment and then reaches the arms to the grips on the car's wheel. [`DriverRig`] holds the skeleton, the clips and the
//! skin of every vertex of the figure; [`DriverRig::pose`] makes the pose for a steering input or a look back, and
//! [`DriverRig::posed`] moves the figure's vertices to it.

use chameleon_anim::{AnimationList, Deltas, Pose, Skeleton, Skin, ik};
use chameleon_bundle::{Bundle, Pool, ResourceId, ResourceKind};
use chameleon_gfx::{Element, Model, Renderable, Vertex, usage};

use crate::Lod;
use crate::driver::{Driver, DriverSetup, MODEL_AT};
use crate::error::{Error, Result};
use crate::lamps::read_groups;
use crate::spec::find_spec;
use crate::vehicle::{Surfaces, expect};

/// Where a `CharacterSpec` keeps the imports of its skeleton and its animation list.
const SKELETON_AT: u32 = 0x04;
const ANIMATIONS_AT: u32 = 0x08;

/// The clip of the animation list that the game's driver state machine scrubs with the steering: 31 keys, one second, from
/// full left (key 0) to full right (key 30). **[exe]** for the use, **[unconfirmed]** for its name.
pub const STEERING_CLIP: usize = 3;
/// The clip the state machine plays for a long look over the shoulder: 112 keys, 3.7 s, the head turned through the middle of it.
pub const LOOK_BACK_CLIP: usize = 19;
/// The time in the look-back clip (seconds) where the head is turned furthest.
pub const LOOK_BACK_TIME: f32 = 50.0 / 30.0;

/// Where the two hands are put, in the figure's own space (the seat taken off the car's space), metres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grips {
    pub left: [f32; 3],
    pub right: [f32; 3],
}

/// What the driver is doing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DriverPose {
    /// Both hands on the wheel, the body leaning with the steering input: -1 full left, 1 full right.
    Steering(f32),
    /// The body turned to look back over the shoulder, as for reversing.
    LookBack,
}

/// The skeleton, clips and skin of a driver figure.
#[derive(Debug, Clone)]
pub struct DriverRig {
    pub skeleton: Skeleton,
    pub animations: AnimationList,
    /// For each level of detail, each mesh of [`Driver::lods`] and each of its vertices: the four bones and weights.
    pub skins: Vec<Vec<Vec<Skin>>>,
    /// The shoulder, elbow and wrist bones of the left and of the right arm.
    pub(crate) arms: [[usize; 3]; 2],
}

fn anim_error(error: chameleon_anim::Error) -> Error {
    Error::Spec(error.to_string())
}

/// The rig of the character `character`. `pool` must hold the character bundle and the shader bundle.
pub fn load_rig(pool: &Pool, character: u32) -> Result<DriverRig> {
    let id = ResourceId::from_parts(character, 0, 0, 1).0;
    let spec = expect(pool, id, ResourceKind::CharacterSpec)?;
    let import = |offset: u32, what: &str| {
        let found = spec.imports.iter().find(|import| import.offset == offset).map(|import| import.id);
        found.ok_or_else(|| Error::Spec(format!("character {character:X} has no {what}")))
    };
    let skeleton = expect(pool, import(SKELETON_AT, "skeleton")?, ResourceKind::Skeleton)?;
    let skeleton = Skeleton::parse(&skeleton.blocks[0]).map_err(anim_error)?;
    let animations = expect(pool, import(ANIMATIONS_AT, "animation list")?, ResourceKind::AnimationList)?;
    let animations = AnimationList::parse(&animations.blocks[0]).map_err(anim_error)?;
    let find = |name: &str| skeleton.find(name).ok_or_else(|| Error::Spec(format!("the skeleton has no {name}")));
    let arms = [
        [find("LeftArm")?, find("LeftForeArm")?, find("LeftHand")?],
        [find("RightArm")?, find("RightForeArm")?, find("RightHand")?],
    ];
    let skins = read_skins(pool, import(MODEL_AT, "model")?)?;
    Ok(DriverRig { skeleton, animations, skins, arms })
}

/// The skin of every vertex of every mesh of every level of a model, in the order [`crate::driver::load_driver`] loads them.
fn read_skins(pool: &Pool, model: u64) -> Result<Vec<Vec<Vec<Skin>>>> {
    let model = expect(pool, model, ResourceKind::Model)?;
    let model = Model::parse(&model.blocks[0], &model.imports)?;
    let surfaces = Surfaces::default();
    let mut levels = Vec::with_capacity(model.lods.len());
    for &lod in &model.lods {
        let resource = expect(pool, lod, ResourceKind::Renderable)?;
        let renderable = Renderable::parse(&resource.blocks[0], &resource.imports)?;
        let mut meshes = Vec::with_capacity(renderable.meshes.len());
        for mesh in &renderable.meshes {
            let Some(surface) = surfaces.get(pool, mesh.material)? else { continue };
            let descriptor = &surface.0;
            let bytes = mesh.vertex_bytes(&resource.blocks[1])?;
            let stride = descriptor.stride().max(1);
            let find = |wanted: u8| descriptor.elements.iter().find(|e| e.usage() == wanted && e.index() == 0);
            let (bones, weights) = (find(usage::BONES), find(usage::WEIGHTS));
            let skins = (0..bytes.len() / stride)
                .map(|vertex| {
                    let at = |element: Option<&Element>| -> [u8; 4] {
                        let Some(element) = element else { return [0; 4] };
                        let start = vertex * stride + element.offset as usize;
                        bytes.get(start..start + 4).and_then(|b| b.try_into().ok()).unwrap_or([0; 4])
                    };
                    Skin { bones: at(bones), weights: at(weights) }
                })
                .collect();
            meshes.push(skins);
        }
        levels.push(meshes);
    }
    Ok(levels)
}

/// The hand grips on the wheel of the car whose bundle is `bundle` and whose driver setup is `setup`, in the figure's space
/// (the car's space less `seat`). The left hand is the grip on the left (larger x). `None` when the car names no group of two
/// grips.
pub fn driver_grips(bundle: &Bundle, setup: &DriverSetup, seat: [f32; 3]) -> Option<Grips> {
    let spec = bundle.resource_by_id(find_spec(bundle)?).ok()?;
    let groups = read_groups(&spec.blocks[0]).ok()?;
    let group = groups.iter().find(|group| group.id == setup.grip_group && group.locators.len() == 2)?;
    let in_figure = |position: [f32; 3]| [0, 1, 2].map(|k| position[k] - seat[k]);
    let (a, b) = (group.locators[0].position, group.locators[1].position);
    let (left, right) = if a[0] >= b[0] { (a, b) } else { (b, a) };
    Some(Grips { left: in_figure(left), right: in_figure(right) })
}

impl DriverRig {
    /// The bone deltas of `pose`, before the arms are reached to the grips.
    fn deltas(&self, pose: DriverPose) -> Deltas {
        let mut deltas = self.skeleton.rest_deltas();
        let (clip, time) = match pose {
            DriverPose::Steering(steer) => {
                let clip = &self.animations.clips[STEERING_CLIP.min(self.animations.clips.len() - 1)];
                (clip, (steer.clamp(-1.0, 1.0) * 0.5 + 0.5) * clip.seconds())
            }
            DriverPose::LookBack => {
                (&self.animations.clips[LOOK_BACK_CLIP.min(self.animations.clips.len() - 1)], LOOK_BACK_TIME)
            }
        };
        self.skeleton.sample(clip, time, &mut deltas);
        deltas
    }

    /// The pose for `pose`. A steering pose with `grips` has both arms reached to them; a look back is the clip as it is.
    pub fn pose(&self, pose: DriverPose, grips: Option<Grips>) -> Pose {
        let mut deltas = self.deltas(pose);
        let mut posed = Pose::new(self.skeleton.bones.len());
        self.skeleton.pose(&deltas, &mut posed);
        let (DriverPose::Steering(_), Some(grips)) = (pose, grips) else { return posed };
        for (chain, target) in self.arms.into_iter().zip([grips.left, grips.right]) {
            ik::reach(&self.skeleton, &mut deltas, &mut posed, chain, target);
        }
        posed
    }

    /// The first `levels` levels of detail of `driver` with every vertex moved to `pose` and then by `seat`.
    pub fn posed(&self, driver: &Driver, pose: &Pose, seat: [f32; 3], levels: usize) -> Vec<Lod> {
        let mut lods: Vec<Lod> = driver.lods.iter().take(levels).cloned().collect();
        for (lod, skins) in lods.iter_mut().zip(&self.skins) {
            for (mesh, skins) in lod.iter_mut().zip(skins) {
                for (vertex, skin) in mesh.vertices.iter_mut().zip(skins) {
                    move_vertex(vertex, self.skeleton.skin_point(pose, skin, vertex.position), seat);
                }
            }
        }
        lods
    }
}

fn move_vertex(vertex: &mut Vertex, moved: [f32; 3], seat: [f32; 3]) {
    vertex.position = [0, 1, 2].map(|k| moved[k] + seat[k]);
}
