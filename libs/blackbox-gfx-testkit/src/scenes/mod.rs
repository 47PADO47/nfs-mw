//! The synthetic scenes every renderer is checked against.
//!
//! A scene is a camera, some procedural meshes and textures, and optionally an effect layer and a UI
//! layer. [`capture_scene`] builds one through a [`RenderBackend`], captures a frame and cleans up, so
//! the same call works for any renderer and any set of graphics settings.

mod alpha_cards;
mod blend_stack;
pub mod depth_probe;
mod effects;
mod glossy;
mod grid;
mod sky_dome;
#[cfg(test)]
mod tests;
mod ui;

use blackbox_gfx::{
    EffectLayer, FrameParams, GlossyMaterial, GlossyMaterialHandle, Instance, MeshHandle, RenderBackend, RenderError,
    RgbaImage, TextureHandle, UiLayer, UiTextureId, UiTexturePatch,
};

use crate::mesh::BuiltMesh;
use crate::metrics::Tolerance;
use crate::texture::{Image, Storage, upload};

/// The scenes. Each exercises one part of the renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SceneId {
    /// An opaque ground grid with BC1 and RGBA textures, vertex colours and a fog ramp, plus lit boxes.
    Grid,
    /// Alpha-tested foliage cards, overlapping and crossing, in front of a wall.
    AlphaCards,
    /// Alpha-blended and additive quads in a fixed submission order over an opaque background.
    BlendStack,
    /// A sky dome 9.7 km away behind fogged ground and buildings.
    SkyDome,
    /// Pairs of quads from 2 m to 9 km that cross or sit 0.1 % apart in depth: reverse-Z precision.
    DepthProbe,
    /// Two glossy spheres lit by the rig and reflecting the environment.
    GlossySphere,
    /// The effect layer: ground surfaces, soft particles, glows, streaks and textured effects.
    Effects,
    /// The 2D UI layer: opaque and translucent panels, a texture, a clip rectangle, vertex colours.
    Ui,
}

impl SceneId {
    pub const ALL: [SceneId; 8] = [
        SceneId::Grid,
        SceneId::AlphaCards,
        SceneId::BlendStack,
        SceneId::SkyDome,
        SceneId::DepthProbe,
        SceneId::GlossySphere,
        SceneId::Effects,
        SceneId::Ui,
    ];

    /// A short stable name, used in digest constants and messages.
    pub fn name(self) -> &'static str {
        match self {
            Self::Grid => "grid",
            Self::AlphaCards => "alpha_cards",
            Self::BlendStack => "blend_stack",
            Self::SkyDome => "sky_dome",
            Self::DepthProbe => "depth_probe",
            Self::GlossySphere => "glossy_sphere",
            Self::Effects => "effects",
            Self::Ui => "ui",
        }
    }

    /// How closely two different renderers are expected to agree on this scene.
    pub fn fidelity(self) -> Fidelity {
        match self {
            Self::Grid | Self::AlphaCards | Self::SkyDome | Self::DepthProbe => Fidelity::Strict,
            Self::GlossySphere => Fidelity::Glossy,
            Self::BlendStack | Self::Effects => Fidelity::Blended,
            Self::Ui => Fidelity::Ui,
        }
    }

    /// Build the scene through `backend` for a `size` (width, height) image.
    pub fn build(self, backend: &mut dyn RenderBackend, size: [u32; 2]) -> BuiltScene {
        let mut cx = Cx::new(backend, size);
        let parts = match self {
            Self::Grid => grid::build(&mut cx),
            Self::AlphaCards => alpha_cards::build(&mut cx),
            Self::BlendStack => blend_stack::build(&mut cx),
            Self::SkyDome => sky_dome::build(&mut cx),
            Self::DepthProbe => depth_probe::build(&mut cx),
            Self::GlossySphere => glossy::build(&mut cx),
            Self::Effects => effects::build(&mut cx),
            Self::Ui => ui::build(&mut cx),
        };
        BuiltScene {
            frame: parts.frame,
            instances: parts.instances,
            effects: parts.effects,
            ui: parts.ui,
            resources: cx.res,
        }
    }
}

/// How closely two renderers should agree, from the plan's tolerance table (out of 255).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fidelity {
    /// Opaque, alpha-test, sky, fog and depth: same pixels within rounding.
    Strict,
    /// Glossy shading: same maths, different float rounding in the lighting.
    Glossy,
    /// Blending and effects: a renderer that blends in linear space differs by design.
    Blended,
    /// The UI pass: the same shared pass, off by at most one.
    Ui,
}

impl Fidelity {
    pub fn tolerance(self) -> Tolerance {
        match self {
            Self::Strict => Tolerance::new(4, 0.5, 2),
            Self::Glossy => Tolerance { max: 255, mean: 1.5, p99: 8 },
            Self::Blended => Tolerance { max: 255, mean: 4.0, p99: 24 },
            Self::Ui => Tolerance { max: 1, mean: 255.0, p99: 255 },
        }
    }
}

/// What a scene's builder returns.
pub(crate) struct Parts {
    pub frame: FrameParams,
    pub instances: Vec<Instance>,
    pub effects: EffectLayer,
    pub ui: UiLayer,
}

impl Parts {
    pub(crate) fn world(frame: FrameParams, instances: Vec<Instance>) -> Self {
        Self { frame, instances, effects: EffectLayer::default(), ui: UiLayer::default() }
    }
}

/// The backend resources a scene created, so they can be released afterwards.
#[derive(Debug, Default)]
pub struct Resources {
    textures: Vec<TextureHandle>,
    meshes: Vec<MeshHandle>,
    glossy: Vec<GlossyMaterialHandle>,
    ui_textures: Vec<UiTextureId>,
}

/// The builder's view of the backend: creates resources and remembers them.
pub(crate) struct Cx<'a> {
    pub backend: &'a mut dyn RenderBackend,
    pub aspect: f32,
    pub size: [u32; 2],
    res: Resources,
}

impl<'a> Cx<'a> {
    fn new(backend: &'a mut dyn RenderBackend, size: [u32; 2]) -> Self {
        let aspect = size[0] as f32 / size[1].max(1) as f32;
        Self { backend, aspect, size, res: Resources::default() }
    }

    pub fn texture(&mut self, label: &str, image: &Image, storage: Storage) -> TextureHandle {
        let handle = upload(self.backend, label, image, storage);
        self.res.textures.push(handle);
        handle
    }

    pub fn mesh(&mut self, label: &str, mesh: &BuiltMesh) -> MeshHandle {
        let handle = mesh.upload(self.backend, label);
        self.res.meshes.push(handle);
        handle
    }

    pub fn glossy(&mut self, material: &GlossyMaterial) -> GlossyMaterialHandle {
        let handle = self.backend.create_glossy_material(material);
        self.res.glossy.push(handle);
        handle
    }

    /// Create (or fully replace) a UI texture.
    pub fn ui_texture(&mut self, id: u64, size: [u32; 2], rgba: &[u8]) -> UiTextureId {
        let id = UiTextureId(id);
        self.backend.update_ui_texture(&UiTexturePatch { id, offset: None, size, rgba });
        self.res.ui_textures.push(id);
        id
    }
}

/// A scene built on a backend: draw it with `frame` and `instances`, with `effects` and `ui` set.
pub struct BuiltScene {
    pub frame: FrameParams,
    pub instances: Vec<Instance>,
    pub effects: EffectLayer,
    pub ui: UiLayer,
    resources: Resources,
}

impl BuiltScene {
    /// Hand the scene's layers to the backend.
    pub fn apply_layers(&self, backend: &mut dyn RenderBackend) {
        backend.set_effects(&self.effects);
        backend.set_ui_layer(self.ui.clone());
    }

    /// Destroy the scene's resources and clear the layers it set.
    pub fn release(self, backend: &mut dyn RenderBackend) {
        backend.set_effects(&EffectLayer::default());
        backend.set_ui_layer(UiLayer::default());
        let Resources { textures, meshes, glossy, ui_textures } = self.resources;
        meshes.into_iter().for_each(|h| backend.destroy_mesh(h));
        textures.into_iter().for_each(|h| backend.destroy_texture(h));
        glossy.into_iter().for_each(|h| backend.destroy_glossy_material(h));
        ui_textures.into_iter().for_each(|id| backend.free_ui_texture(id));
    }
}

/// How many frames to render while waiting for a capture before giving up.
const MAX_WAIT_FRAMES: usize = 600;

/// Build `scene`, capture one frame of `size` (width, height) with the backend's current graphics
/// settings, release the scene and return the image.
///
/// A backend that completes captures at once returns on the first poll. One that needs frames to pass
/// is given them: the scene is rendered between polls.
pub fn capture_scene(
    backend: &mut dyn RenderBackend,
    scene: SceneId,
    size: [u32; 2],
) -> Result<RgbaImage, RenderError> {
    let built = scene.build(backend, size);
    built.apply_layers(backend);
    let result = capture_built(backend, &built, size);
    built.release(backend);
    result
}

fn capture_built(
    backend: &mut dyn RenderBackend,
    built: &BuiltScene,
    size: [u32; 2],
) -> Result<RgbaImage, RenderError> {
    let id = backend.request_capture(size, &built.frame, &built.instances)?;
    for _ in 0..MAX_WAIT_FRAMES {
        if let Some(result) = backend.poll_capture(id) {
            return result;
        }
        backend.render(&built.frame, &built.instances)?;
    }
    Err(RenderError::Device(format!("the capture did not complete in {MAX_WAIT_FRAMES} frames")))
}
