//! The world material: `BlackboxMaterial`, specialised per shading model and blend mode.
//!
//! One Bevy material asset exists per (texture, blend, shading, glossy material); see
//! `docs/bevy-backend.md`, "Colour" and "Draw order", for why the shader works in gamma space and what
//! Bevy's sorted blending changes.

pub(crate) mod color;
pub mod glossy;
mod prepass;
pub mod scene;
use prepass::{shader as prepass_shader, specialize as specialize_prepass};

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
use bevy_render::storage::ShaderBuffer;
use bevy_shader::ShaderRef;
use blackbox_gfx::{BlendMode, FrameParams, Shading};

use crate::mesh::ATTRIBUTE_COLOR_BGRA;
pub use glossy::{GlossyUniform, RigUniform};
pub use scene::{SCENE_BUFFER, SceneUniform};

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
    /// Authored ambient plus direct scenery lighting, with atmospheric extinction.
    World,
    /// Tree lighting with texture-only alpha and different atmospheric attenuation.
    Tree,
    /// Doubled diffuse/vertex RGB without lighting or atmosphere.
    WorldNoFog,
    /// Like `WorldNoFog`, with doubled diffuse/vertex alpha.
    WorldPrelit,
}

impl ShadingKind {
    /// The model that draws `shading`.
    pub fn of(shading: Shading) -> Self {
        match shading {
            Shading::Prelit => Self::Prelit,
            Shading::Sky => Self::Sky,
            Shading::SkyLayer(_) => Self::Sky,
            Shading::Lit => Self::Lit,
            Shading::Glossy(_) => Self::Glossy,
            Shading::World => Self::World,
            // Registration returns None on this backend. Keep externally supplied
            // unsupported handles on the same explicit legacy fallback.
            Shading::WorldNormal(_) | Shading::WorldWindow(_) => Self::Prelit,
            Shading::Tree => Self::Tree,
            Shading::WorldNoFog => Self::WorldNoFog,
            Shading::WorldPrelit => Self::WorldPrelit,
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

/// The frame's fog and light, the first half of [`SceneUniform`]: shared by every material through
/// [`SCENE_BUFFER`], rewritten whenever a frame's values differ from the last.
#[derive(Debug, Clone, Copy, PartialEq, ShaderType)]
pub struct Params {
    /// Fog colour (the frame's clear colour), gamma space.
    pub fog_color: Vec4,
    /// x = start, y = end.
    pub fog_range: Vec4,
    /// The direction the light travels, in Bevy's axes.
    pub light_dir: Vec4,
    /// x = use the ACES reference-colour bridge.
    pub tone_map: Vec4,
    /// Authored RGB and intensity; presence is separate in `surface_diffuse.w`.
    pub surface_ambient: Vec4,
    pub surface_diffuse: Vec4,
    /// Rayleigh RGB and atmospheric enable flag.
    pub scatter_rayleigh: Vec4,
    /// Mie RGB and anisotropy.
    pub scatter_mie: Vec4,
    /// Sun direction in Bevy axes, and authored distance scale.
    pub to_sun: Vec4,
    /// x = original scenery, y = authored in-scattering multiplier.
    pub scenery: Vec4,
}

impl Params {
    /// The parameters a frame asks for.
    pub fn of(frame: &FrameParams) -> Self {
        let [start, end] = frame.fog_range();
        let [r, g, b] = frame.clear_color;
        let light = crate::axes::point(frame.light_dir);
        let mut out = Self {
            fog_color: Vec4::new(r, g, b, 1.0),
            fog_range: Vec4::new(start, end, 0.0, 0.0),
            light_dir: Vec4::new(light.x, light.y, light.z, 0.0),
            tone_map: Vec4::ZERO,
            ..Self::default()
        };
        if let Some(light) = frame.surface_light {
            out.surface_ambient =
                Vec4::from_array([light.ambient[0], light.ambient[1], light.ambient[2], light.ambient_alpha]);
            out.surface_diffuse = Vec4::from_array([light.diffuse[0], light.diffuse[1], light.diffuse[2], 1.0]);
        }
        if let Some(scatter) = frame.scattering {
            out.scatter_rayleigh =
                Vec4::from_array([scatter.rayleigh[0], scatter.rayleigh[1], scatter.rayleigh[2], 1.0]);
            out.scatter_mie = Vec4::from_array([scatter.mie[0], scatter.mie[1], scatter.mie[2], scatter.g]);
            let sun = crate::axes::point(scatter.to_sun);
            out.to_sun = Vec4::new(sun.x, sun.y, sun.z, scatter.distance_scale);
            out.scenery.y = scatter.in_scatter_scale;
        }
        out
    }

    pub fn with_tonemap(frame: &FrameParams, tonemap: blackbox_gfx::Tonemap) -> Self {
        Self {
            tone_map: Vec4::new(u8::from(tonemap == blackbox_gfx::Tonemap::Aces).into(), 0.0, 0.0, 0.0),
            ..Self::of(frame)
        }
    }

    pub fn with_original_world(mut self, enabled: bool) -> Self {
        self.scenery.x = u8::from(enabled).into();
        self
    }
}

impl Default for Params {
    fn default() -> Self {
        Self {
            fog_color: Vec4::new(0.0, 0.0, 0.0, 1.0),
            fog_range: Vec4::new(f32::MAX, f32::MAX, 0.0, 0.0),
            light_dir: Vec4::new(0.0, -1.0, 0.0, 0.0),
            tone_map: Vec4::ZERO,
            surface_ambient: Vec4::new(0.35, 0.35, 0.35, 1.0),
            surface_diffuse: Vec4::new(0.65, 0.65, 0.65, 0.0),
            scatter_rayleigh: Vec4::ZERO,
            scatter_mie: Vec4::ZERO,
            to_sun: Vec4::new(0.0, 1.0, 0.0, 1.0),
            scenery: Vec4::new(0.0, 1.0, 0.0, 0.0),
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
    /// The scene's fog, light and lighting rig, shared by every material; see `scene.rs`.
    #[storage(0, read_only)]
    pub scene: Handle<ShaderBuffer>,
    #[texture(1)]
    #[sampler(2)]
    pub texture: Option<Handle<Image>>,
    /// This material's own shading constants, when it is [`ShadingKind::Glossy`]; unused otherwise.
    #[uniform(3)]
    pub glossy: GlossyUniform,
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
    bevy_shader::load_shader_library!(app, "prepass.wesl");
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

    /// Depth and motion vectors for the camera's prepasses (FSR 3, DLSS and TAA need them) and for shadow maps.
    /// Only opaque and alpha-tested draws are queued; blended and additive ones never reach a prepass.
    fn enable_prepass() -> bool {
        true
    }

    fn enable_shadows() -> bool {
        true
    }

    fn prepass_vertex_shader() -> ShaderRef {
        prepass_shader()
    }

    fn prepass_fragment_shader() -> ShaderRef {
        prepass_shader()
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let BlackboxKey { shading, blend } = key.bind_group_data;
        if descriptor.vertex.shader_defs.iter().any(|def| def == &"PREPASS_PIPELINE".into()) {
            return specialize_prepass(descriptor, layout, shading, blend);
        }
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
        if shading != ShadingKind::Sky && blend != BlendKind::Additive {
            defs.push("RECEIVE_SHADOWS".into());
        }
        if shading == ShadingKind::Glossy {
            defs.push("GLOSSY".into());
        } else if shading != ShadingKind::Lit {
            defs.push("PRELIT".into());
        }
        if shading != ShadingKind::Sky {
            defs.push("FOG".into());
        }
        let family = match shading {
            ShadingKind::World => Some("WORLD"),
            ShadingKind::Tree => Some("TREE"),
            ShadingKind::WorldNoFog => Some("WORLD_NO_FOG"),
            ShadingKind::WorldPrelit => Some("WORLD_PRELIT"),
            _ => None,
        };
        if let Some(family) = family {
            descriptor.vertex.shader_defs.push(family.into());
            defs.push(family.into());
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
mod prepass_tests;

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
            scattering: None,
            surface_light: None,
            camera_cut: false,
            shadow_tuning: None,
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
        assert_eq!(ShadingKind::of(Shading::World), ShadingKind::World);
        assert_eq!(
            ShadingKind::of(Shading::WorldNormal(blackbox_gfx::NormalMaterialHandle::from_raw(1))),
            ShadingKind::Prelit
        );
        assert_eq!(ShadingKind::of(Shading::Tree), ShadingKind::Tree);
        assert_eq!(ShadingKind::of(Shading::WorldNoFog), ShadingKind::WorldNoFog);
        assert_eq!(ShadingKind::of(Shading::WorldPrelit), ShadingKind::WorldPrelit);
        assert_eq!(
            ShadingKind::of(Shading::Glossy(blackbox_gfx::GlossyMaterialHandle::from_raw(1))),
            ShadingKind::Glossy
        );
    }

    #[test]
    fn the_key_is_the_shading_and_the_blend() {
        let material = BlackboxMaterial {
            scene: SCENE_BUFFER,
            texture: None,
            glossy: GlossyUniform::default(),
            environment: None,
            shading: ShadingKind::Sky,
            blend: BlendKind::Additive,
        };
        assert_eq!(BlackboxKey::from(&material), BlackboxKey { shading: ShadingKind::Sky, blend: BlendKind::Additive });
    }

    #[test]
    fn authored_intensity_and_atmosphere_are_distinct_from_presence_and_style() {
        let mut frame = frame(None);
        frame.surface_light = Some(blackbox_gfx::SurfaceLight {
            shadow_strength: 1.0,
            specular: [0.0; 3],
            specular_power: 1.0,
            reflection_scale: 0.0,
            reflective_specular_scale: 1.0,
            ambient: [0.3, 0.2, 0.1],
            ambient_alpha: 0.4,
            diffuse: [0.8, 0.7, 0.6],
        });
        frame.scattering = Some(blackbox_gfx::Scattering {
            rayleigh: [0.01, 0.02, 0.03],
            mie: [0.04, 0.05, 0.06],
            g: 0.7,
            to_sun: Vec3::Z,
            distance_scale: 0.8,
            in_scatter_scale: 0.9,
        });
        let params = Params::of(&frame);
        assert_eq!(params.surface_ambient.w, 0.4);
        assert_eq!(params.surface_diffuse.w, 1.0);
        assert_eq!(params.scatter_rayleigh.w, 1.0);
        assert_eq!(params.to_sun, Vec4::new(0.0, 1.0, 0.0, 0.8));
        assert_eq!(params.scenery.y, 0.9);
        assert_eq!(params.scenery.x, 0.0);
        let original = params.with_original_world(true);
        assert_eq!(original.scenery.x, 1.0);
        assert_eq!(original.surface_ambient, params.surface_ambient);
        assert_eq!(original.scenery.y, params.scenery.y);
    }
}
