//! The cameras: the one on screen, and the short-lived ones that capture to an image.
//!
//! A camera takes its placement from the frame's view matrix (in Bevy's axes, see [`crate::axes`]) and its
//! lens from the frame's projection. The projection is Bevy's own reverse-Z infinite perspective, which is
//! exactly the matrix the games' callers build; the aspect ratio follows the render target.

use bevy_anti_alias::fxaa::Fxaa;
use bevy_camera::{
    Camera, Camera3d, Camera3dDepthTextureUsage, ClearColorConfig, MainPassResolutionOverride, PerspectiveProjection,
    Projection, RenderTarget,
};
use bevy_color::Color;
use bevy_ecs::bundle::Bundle;
use bevy_ecs::entity::Entity;
use bevy_ecs::system::Commands;
use bevy_math::UVec2;
use bevy_render::camera::MipBias;
use bevy_render::view::{DebandDither, Msaa, Tonemapping};
use bevy_transform::components::GlobalTransform;
use blackbox_gfx::{FrameParams, Projection as GameProjection};

use crate::ops::CameraSettings;

/// Far plane Bevy uses only for culling; the projection itself has none.
const FAR: f32 = 1.0e6;

/// The Bevy camera pose of a frame.
pub fn pose(frame: &FrameParams) -> GlobalTransform {
    let world = crate::axes::camera_world(frame.view);
    GlobalTransform::from(bevy_math::Mat4::from_cols_array(&world.to_cols_array()))
}

/// The lens of a frame. 2D frames (an identity projection) carry no scene, so any lens will do.
pub fn lens(frame: &FrameParams) -> Projection {
    let perspective = match frame.projection {
        GameProjection::PerspectiveInfiniteReverse { fov_y, aspect, near } => {
            PerspectiveProjection { fov: fov_y, aspect_ratio: aspect, near, far: FAR, ..Default::default() }
        }
        GameProjection::Identity => PerspectiveProjection::default(),
    };
    Projection::Perspective(perspective)
}

/// The clear colour: the frame's gamma-space values, which the sRGB target encodes back to the same bytes.
pub fn clear(frame: &FrameParams) -> ClearColorConfig {
    let [r, g, b] = frame.clear_color;
    ClearColorConfig::Custom(Color::srgb(r, g, b))
}

/// The pixel size the scene is drawn at, when it is not the target's.
pub fn render_override(target: [u32; 2], settings: &CameraSettings) -> Option<UVec2> {
    if settings.render_scale >= 1.0 {
        return None;
    }
    let (w, h) = blackbox_gfx::scaled_size((target[0], target[1]), settings.render_scale);
    Some(UVec2::new(w, h))
}

/// Everything a new camera needs; later frames overwrite the pose, lens, clear colour and bias.
pub fn bundle(frame: &FrameParams, target: RenderTarget, settings: &CameraSettings) -> impl Bundle {
    (
        Camera3d {
            // The soft-particle pass samples the finished depth over a second pass; see
            // `systems::effects::effects_soft_pass` and docs/bevy-backend.md ("PR 9").
            depth_texture_usages: Camera3dDepthTextureUsage::from(
                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            ),
            ..Camera3d::default()
        },
        Camera { clear_color: clear(frame), ..Default::default() },
        target,
        lens(frame),
        pose(frame),
        // The world is shaded in the material; Bevy must not tone map, dither or multisample it.
        Tonemapping::None,
        DebandDither::Disabled,
        Msaa::Off,
        MipBias(settings.mip_bias),
    )
}

/// Overwrite what changes from frame to frame on an existing camera.
pub fn follow(commands: &mut Commands, camera: Entity, frame: &FrameParams, settings: &CameraSettings) {
    let mut entity = commands.entity(camera);
    entity.insert((lens(frame), pose(frame), MipBias(settings.mip_bias)));
    entity.entry::<Camera>().and_modify({
        let clear = clear(frame);
        move |mut camera| camera.clear_color = clear
    });
}

/// Turn FXAA and the reduced main-pass size on or off.
pub fn set_post(commands: &mut Commands, camera: Entity, settings: &CameraSettings, target: [u32; 2]) {
    let mut entity = commands.entity(camera);
    match settings.fxaa {
        true => entity.insert(Fxaa::default()),
        false => entity.remove::<Fxaa>(),
    };
    match render_override(target, settings) {
        Some(size) => entity.insert(MainPassResolutionOverride(size)),
        None => entity.remove::<MainPassResolutionOverride>(),
    };
}
