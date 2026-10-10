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
    CaptureId, FrameParams, GlossyMaterialHandle, Instance, MeshHandle, RenderError, RgbaImage, TextureHandle,
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

/// Settings the facade passes on to the cameras.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraSettings {
    pub vsync: bool,
    /// Added to the texture LOD (the render scale's suggested bias).
    pub mip_bias: f32,
    pub fxaa: bool,
    /// The fraction of the surface the scene is drawn at (bilinear upscaling when below 1).
    pub render_scale: f32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self { vsync: true, mip_bias: 0.0, fxaa: false, render_scale: 1.0 }
    }
}

/// The queue's contents.
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
