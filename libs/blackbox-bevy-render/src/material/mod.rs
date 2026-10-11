//! The world material: `BlackboxMaterial`, specialised per shading model and blend mode.
//!
//! One Bevy material asset exists per (texture, blend, shading, glossy material); see
//! `docs/bevy-backend.md`, "Colour" and "Draw order", for why the shader works in gamma space and what
//! Bevy's sorted blending changes.

pub mod glossy;

use bevy_asset::{Asset, Handle};
use bevy_image::Image;
use bevy_math::Vec4;
use bevy_mesh::{Mesh, MeshVertexBufferLayoutRef};
use bevy_pbr::{Material, MaterialPipeline, MaterialPipelineKey};
use bevy_reflect::TypePath;
use bevy_render::render_resource::{
    AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState, ColorWrites, RenderPipelineDescriptor,
    ShaderType, SpecializedMeshPipelineError,
};
use bevy_shader::ShaderRef;
use blackbox_gfx::{BlendMode, FrameParams, Shading};

use crate::mesh::ATTRIBUTE_COLOR_BGRA;
pub use glossy::{GlossyUniform, RigUniform};

/// The shading models the material is specialised for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShadingKind {
    /// Texture x vertex colour x 2, fogged.
    Prelit,
    /// Texture x vertex colour x (ambient + sun), fogged.
    Lit,
    /// Like `Prelit` but never fogged.
    Sky,
    /// Three lights, a sun highlight and an environment reflection; see `glossy.rs`.
    Glossy,
}

impl ShadingKind {
    /// The model that draws `shading`.
    pub fn of(shading: Shading) -> Self {
        match shading {
            Shading::Prelit => Self::Prelit,
            Shading::Sky => Self::Sky,
            Shading::Lit => Self::Lit,
            Shading::Glossy(_) => Self::Glossy,
        }
    }
}

/// How a draw blends; [`BlendMode`] with `Hash`, so it can be part of a cache key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlendKind {
    Opaque,
    AlphaTest,
    AlphaBlend,
    Additive,
}

impl From<BlendMode> for BlendKind {
    fn from(mode: BlendMode) -> Self {
        match mode {
            BlendMode::Opaque => Self::Opaque,
            BlendMode::AlphaTest => Self::AlphaTest,
            BlendMode::AlphaBlend => Self::AlphaBlend,
            BlendMode::Additive => Self::Additive,
        }
    }
}

/// What the shader reads besides the texture: constant for a scene, so every material of a scene holds the
/// same copy and the copies are rewritten only when the frame's fog or light changes.
#[derive(Debug, Clone, Copy, PartialEq, ShaderType)]
pub struct Params {
    /// Fog colour (the frame's clear colour), gamma space.
    pub fog_color: Vec4,
    /// x = start, y = end.
    pub fog_range: Vec4,
    /// The direction the light travels, in Bevy's axes.
    pub light_dir: Vec4,
}

impl Params {
    /// The parameters a frame asks for.
    pub fn of(frame: &FrameParams) -> Self {
        let [start, end] = frame.fog_range();
        let [r, g, b] = frame.clear_color;
        let light = crate::axes::point(frame.light_dir);
        Self {
            fog_color: Vec4::new(r, g, b, 1.0),
            fog_range: Vec4::new(start, end, 0.0, 0.0),
            light_dir: Vec4::new(light.x, light.y, light.z, 0.0),
        }
    }
}

impl Default for Params {
    fn default() -> Self {
        Self {
            fog_color: Vec4::new(0.0, 0.0, 0.0, 1.0),
            fog_range: Vec4::new(f32::MAX, f32::MAX, 0.0, 0.0),
            light_dir: Vec4::new(0.0, -1.0, 0.0, 0.0),
        }
    }
}

/// The part of the material that picks a pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlackboxKey {
    pub shading: ShadingKind,
    pub blend: BlendKind,
}

impl From<&BlackboxMaterial> for BlackboxKey {
    fn from(material: &BlackboxMaterial) -> Self {
        Self { shading: material.shading, blend: material.blend }
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bind_group_data(BlackboxKey)]
pub struct BlackboxMaterial {
    #[uniform(0)]
    pub params: Params,
    #[texture(1)]
    #[sampler(2)]
    pub texture: Option<Handle<Image>>,
    /// This material's own shading constants, when it is [`ShadingKind::Glossy`]; unused otherwise.
    #[uniform(3)]
    pub glossy: GlossyUniform,
    /// The lighting rig every glossy material shares, re-synced like `params` whenever it changes.
    #[uniform(4)]
    pub rig: RigUniform,
    #[texture(5, dimension = "cube")]
    #[sampler(6)]
    pub environment: Option<Handle<Image>>,
    pub shading: ShadingKind,
    pub blend: BlendKind,
}

/// `src * alpha + dest`, like the native additive pipeline.
const ADDITIVE: BlendState = BlendState {
    color: BlendComponent {
        src_factor: BlendFactor::SrcAlpha,
        dst_factor: BlendFactor::One,
        operation: BlendOperation::Add,
    },
    alpha: BlendComponent::OVER,
};

/// Loads the material's shader (embedded in this crate) and keeps it loaded. Called once by the plugin.
pub(crate) fn load_shaders(app: &mut bevy_app::App) {
    bevy_shader::load_shader_library!(app, "blackbox.wesl");
}

fn shader() -> ShaderRef {
    ShaderRef::Path(
        bevy_asset::AssetPath::from_path_buf(bevy_asset::embedded_path!("blackbox.wesl")).with_source("embedded"),
    )
}

impl Material for BlackboxMaterial {
    fn vertex_shader() -> ShaderRef {
        shader()
    }

    fn fragment_shader() -> ShaderRef {
        shader()
    }

    fn alpha_mode(&self) -> bevy_material::AlphaMode {
        match self.blend {
            BlendKind::Opaque => bevy_material::AlphaMode::Opaque,
            BlendKind::AlphaTest => bevy_material::AlphaMode::Mask(0.5),
            BlendKind::AlphaBlend => bevy_material::AlphaMode::Blend,
            BlendKind::Additive => bevy_material::AlphaMode::Add,
        }
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let BlackboxKey { shading, blend } = key.bind_group_data;
        descriptor.vertex.buffers = vec![layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
            ATTRIBUTE_COLOR_BGRA.at_shader_location(3),
        ])?];
        // The games' culling per effect is not mapped yet; the native renderer draws both sides.
        descriptor.primitive.cull_mode = None;

        let fragment = descriptor.fragment.as_mut().expect("the material has a fragment shader");
        let defs = &mut fragment.shader_defs;
        if shading == ShadingKind::Glossy {
            defs.push("GLOSSY".into());
        } else if shading != ShadingKind::Lit {
            defs.push("PRELIT".into());
        }
        if shading != ShadingKind::Sky {
            defs.push("FOG".into());
        }
        if blend == BlendKind::AlphaTest {
            defs.push("ALPHA_TEST".into());
        }
        if matches!(blend, BlendKind::Opaque | BlendKind::AlphaTest) {
            defs.push("OPAQUE_ALPHA".into());
        }
        let target = fragment.targets[0].as_mut().expect("the material has a colour target");
        // Colour only: alpha stays what the clear left, so captures are opaque like the native surface.
        target.write_mask = ColorWrites::COLOR;
        match blend {
            BlendKind::Opaque | BlendKind::AlphaTest => target.blend = None,
            BlendKind::AlphaBlend => target.blend = Some(BlendState::ALPHA_BLENDING),
            BlendKind::Additive => target.blend = Some(ADDITIVE),
        }
        if let Some(depth) = descriptor.depth_stencil.as_mut() {
            depth.depth_write_enabled = Some(matches!(blend, BlendKind::Opaque | BlendKind::AlphaTest));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_gfx::{Fog, Projection};
    use glam::{Mat4, Vec3};

    fn frame(fog: Option<Fog>) -> FrameParams {
        FrameParams {
            view: Mat4::IDENTITY,
            projection: Projection::Identity,
            camera_position: Vec3::ZERO,
            light_dir: Vec3::new(0.0, 1.0, -1.0).normalize(),
            clear_color: [0.25, 0.5, 0.75],
            fog,
            camera_cut: false,
        }
    }

    #[test]
    fn the_light_direction_is_in_bevy_axes() {
        let params = Params::of(&frame(None));
        // game (0, 1, -1) travels forward and down; Bevy: (0, -1, -1) is down and into the screen.
        assert!((params.light_dir.y + 0.70710677).abs() < 1e-5);
        assert!((params.light_dir.z + 0.70710677).abs() < 1e-5);
    }

    #[test]
    fn no_fog_is_the_far_away_range() {
        assert_eq!(Params::of(&frame(None)).fog_range.x, f32::MAX);
        let fogged = Params::of(&frame(Some(Fog { start: 8.0, end: 45.0 })));
        assert_eq!((fogged.fog_range.x, fogged.fog_range.y), (8.0, 45.0));
        assert_eq!(fogged.fog_color.truncate(), bevy_math::Vec3::new(0.25, 0.5, 0.75));
    }

    #[test]
    fn shadings_map_to_pipelines() {
        assert_eq!(ShadingKind::of(Shading::Prelit), ShadingKind::Prelit);
        assert_eq!(ShadingKind::of(Shading::Sky), ShadingKind::Sky);
        assert_eq!(ShadingKind::of(Shading::Lit), ShadingKind::Lit);
        assert_eq!(
            ShadingKind::of(Shading::Glossy(blackbox_gfx::GlossyMaterialHandle::from_raw(1))),
            ShadingKind::Glossy
        );
    }

    #[test]
    fn the_key_is_the_shading_and_the_blend() {
        let material = BlackboxMaterial {
            params: Params::default(),
            texture: None,
            glossy: GlossyUniform::default(),
            rig: RigUniform::default(),
            environment: None,
            shading: ShadingKind::Sky,
            blend: BlendKind::Additive,
        };
        assert_eq!(BlackboxKey::from(&material), BlackboxKey { shading: ShadingKind::Sky, blend: BlendKind::Additive });
    }
}
