//! What the apply system remembers between frames.

use std::collections::HashMap;

use bevy_asset::Handle;
use bevy_ecs::entity::Entity;
use bevy_ecs::resource::Resource;
use bevy_image::Image;
use bevy_mesh::Mesh;

use crate::material::{BlackboxMaterial, BlendKind, GlossyUniform, Params, RigUniform, ShadingKind};

/// One draw range of a registered mesh, ready to give an entity.
pub struct RangeEntry {
    pub mesh: Handle<Mesh>,
    /// The caller's texture number, or `None` for the white texture.
    pub texture: Option<usize>,
    pub blend: BlendKind,
    pub shading: ShadingKind,
    /// The glossy material number, when `shading` is [`ShadingKind::Glossy`].
    pub glossy: Option<usize>,
}

/// A registered mesh: one Bevy mesh per draw range.
pub struct MeshEntry {
    pub ranges: Vec<RangeEntry>,
}

/// What picks a material.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaterialKey {
    pub texture: Option<usize>,
    pub blend: BlendKind,
    pub shading: ShadingKind,
    /// The glossy material number, when `shading` is [`ShadingKind::Glossy`].
    pub glossy: Option<usize>,
}

/// One material asset per (texture, blend, shading), shared by every draw that uses the combination.
#[derive(Default)]
pub struct MaterialCache {
    pub by_key: HashMap<MaterialKey, Handle<BlackboxMaterial>>,
    /// The keys that use a texture, so destroying the texture drops its materials without a scan.
    pub by_texture: HashMap<usize, Vec<MaterialKey>>,
}

/// What names a pooled instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PoolKey {
    /// An instance with a stable `InstanceKey`.
    Keyed(u64),
    /// The n-th instance without one in this frame.
    Slot(u32),
}

/// The entities of one placed object: one per draw range of its mesh.
pub struct Placed {
    pub mesh: usize,
    pub entities: Vec<Entity>,
    pub transform: glam::Mat4,
    /// The epoch of the last frame that listed this object.
    pub seen: u64,
    /// Set once the object stopped being listed; it is despawned after a grace period.
    pub hidden_since: Option<u64>,
}

/// The camera that is on screen and the settings last applied to it.
pub struct ScreenCamera {
    pub entity: Entity,
    /// `None` right after spawning: forces [`super::camera::set_post`] to run at least once, so the very
    /// first frame already has the effective post-process components instead of catching up a frame late.
    pub applied: Option<crate::ops::CameraSettings>,
    pub render_size: Option<bevy_math::UVec2>,
    pub vsync: Option<bool>,
}

#[derive(Resource, Default)]
pub struct WorldState {
    pub textures: HashMap<usize, Handle<Image>>,
    /// Texture number to redirect to, keyed by the texture number draws still ask for.
    pub redirects: HashMap<usize, usize>,
    pub meshes: HashMap<usize, MeshEntry>,
    pub materials: MaterialCache,
    /// The parameters every material currently holds.
    pub params: Option<Params>,
    /// Every registered glossy material's shading constants, keyed by its number.
    pub glossy: HashMap<usize, GlossyUniform>,
    /// The lighting rig every glossy material currently reads.
    pub rig: RigUniform,
    /// The environment reflection every glossy material currently reads.
    pub environment: Option<Handle<Image>>,
    /// The `(rig, environment)` every glossy material currently holds, so re-syncing can be skipped
    /// once nothing has changed.
    pub rig_environment_applied: Option<(RigUniform, Option<Handle<Image>>)>,
    pub pool: HashMap<PoolKey, Placed>,
    pub screen: Option<ScreenCamera>,
    pub capture: Option<super::capture::ActiveCapture>,
    /// Counts applied frames; entities not listed for [`GRACE_FRAMES`] of them are despawned.
    pub epoch: u64,
}

/// How many frames an object may be missing before its entities are despawned. Streaming pops tiles in and out
/// at the edge of view; hiding first keeps that from churning entities.
pub const GRACE_FRAMES: u64 = 120;
