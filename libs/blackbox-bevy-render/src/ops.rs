//! The queue between the immediate-mode facade and Bevy's retained world.
//!
//! The game calls `RenderBackend` methods from its own systems, where there is no access to the Bevy world.
//! The facade ([`crate::BevyBackend`]) therefore allocates handles itself and records what it was asked to do
//! as [`Op`]s in a [`Shared`] queue; the [`apply`](crate::apply) system drains the queue in `PostUpdate` of the
//! same tick, before Bevy extracts the world for rendering.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use bevy_ecs::resource::Resource;
use bevy_image::Image;
use blackbox_gfx::{
    CaptureId, DEFAULT_UPSCALE_SHARPNESS, EffectLayer, FrameParams, GlossyMaterialHandle, Instance, MeshHandle,
    PostSettings, RenderError, RgbaImage, TextureHandle, UiLayer, UiTextureId, Upscaler,
};

use crate::material::{GlossyUniform, RigUniform};
use crate::mesh::RangeData;

/// One resource change, applied in the order it was made.
pub enum Op {
    AddTexture {
        handle: TextureHandle,
        image: Box<Image>,
    },
    RemoveTexture(TextureHandle),
    /// Draw `to` wherever `from` is used, or stop redirecting `from` when `to` is `None`.
    SetRedirect {
        from: TextureHandle,
        to: Option<TextureHandle>,
    },
    AddMesh {
        handle: MeshHandle,
        ranges: Vec<RangeData>,
    },
    RemoveMesh(MeshHandle),
    AddGlossyMaterial {
        handle: GlossyMaterialHandle,
        params: GlossyUniform,
    },
    RemoveGlossyMaterial(GlossyMaterialHandle),
    SetLightingRig(RigUniform),
    /// The new environment cube map, built on the calling thread.
    SetEnvironment(Box<Image>),
}

/// Everything one frame is drawn from.
#[derive(Clone)]
pub struct FrameData {
    pub frame: FrameParams,
    pub instances: Vec<Instance>,
}

/// A capture that was requested and not yet answered.
pub struct CaptureRequest {
    pub id: CaptureId,
    pub size: [u32; 2],
    pub data: FrameData,
}

/// A UI texture change, queued for the render world to apply (it owns the pixel data; native's
/// `UiTexturePatch` borrows it, which cannot cross the facade/render-world boundary).
pub enum UiOp {
    Update { id: UiTextureId, offset: Option<[u32; 2]>, size: [u32; 2], rgba: Vec<u8> },
    Free(UiTextureId),
}

/// Capacities the render world's effects pipeline reports once it exists, for `stats()`. Fixed at
/// construction, like native's.
#[derive(Debug, Clone, Copy, Default)]
pub struct EffectCapacities {
    pub surfaces: usize,
    pub particles: usize,
    pub streaks: usize,
}

/// Settings the facade passes on to the cameras: the full effective [`PostSettings`] and [`Upscaler`], not
/// just the FXAA and bilinear subset the spike offered. [`apply::camera`](crate::apply::camera) and
/// [`crate::post`] read this to insert or remove the matching components every time it changes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraSettings {
    pub vsync: bool,
    /// Added to the texture LOD: the render scale's suggested bias, one mip sharper when the
    /// anti-aliasing or the upscaler is temporal (see `suggested_temporal_texture_lod_bias`).
    pub mip_bias: f32,
    /// The fraction of the surface the scene is drawn at (upscaled by `upscaler` when below 1).
    pub render_scale: f32,
    pub post: PostSettings,
    pub upscaler: Upscaler,
    /// FSR 1's RCAS sharpening strength, 0.0 (off) to 1.0 (strongest); meaningless for other upscalers.
    pub upscale_sharpness: f32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            vsync: true,
            mip_bias: 0.0,
            render_scale: 1.0,
            post: PostSettings::default(),
            upscaler: Upscaler::default(),
            upscale_sharpness: DEFAULT_UPSCALE_SHARPNESS,
        }
    }
}

/// The queue's contents. Shared by both worlds: the facade (main world) writes to it, and the render
/// world's Core 3D systems (given the same `Arc` directly, not through Bevy's extract step, so there is
/// no extra frame of latency) read and drain it.
#[derive(Default)]
pub struct Shared {
    pub ops: Vec<Op>,
    /// The newest `render()` call; only the latest frame matters.
    pub frame: Option<FrameData>,
    pub captures: VecDeque<CaptureRequest>,
    /// Captures the world finished, waiting to be polled.
    pub finished: HashMap<u64, Result<RgbaImage, RenderError>>,
    pub settings: CameraSettings,
    /// The surface size the facade was last told about (windows follow Bevy's own size).
    pub surface: [u32; 2],
    /// The latest effect layer, drawn every frame until replaced again.
    pub effects: EffectLayer,
    /// UI texture changes, drained by the render world in the order they were made.
    pub ui_ops: Vec<UiOp>,
    /// The latest UI layer, drawn every frame until replaced again.
    pub ui_layer: UiLayer,
    /// Set once the render world's effects pipeline exists, for `stats()`.
    pub effect_capacities: Option<EffectCapacities>,
}

/// The main-world end of the queue; the facade holds a clone of the same `Arc`.
#[derive(Resource, Clone, Default)]
pub struct BlackboxBridge(pub(crate) Arc<Mutex<Shared>>);

impl BlackboxBridge {
    pub(crate) fn lock(&self) -> MutexGuard<'_, Shared> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn share(&self) -> Arc<Mutex<Shared>> {
        self.0.clone()
    }
}

/// Lock a queue, ignoring poisoning (a panic elsewhere must not hide the frame).
pub(crate) fn lock(shared: &Mutex<Shared>) -> MutexGuard<'_, Shared> {
    shared.lock().unwrap_or_else(PoisonError::into_inner)
}
