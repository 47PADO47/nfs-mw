//! The speed and pursuit radial blur, as a pass over the finished frame: after tone mapping, before the upscale and
//! the HUD, where native draws it (`blackbox-render`'s `gpu/post/radial_blur.rs`). It is the same pass
//! (`blackbox_gpu_passes::RadialBlur`), so the look is the same too. At strength 0 nothing is allocated and nothing
//! is drawn.
//!
//! With a render scale below 1 the scene is drawn into a corner of a texture of the output's size
//! (`post::scale`), so the pass is told how much of the texture holds the image and blurs that part about its own
//! middle.

use bevy_ecs::resource::Resource;
use bevy_ecs::system::{Res, ResMut};
use bevy_render::camera::ExtractedCamera;
use bevy_render::renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery};
use bevy_render::view::ViewTarget;
use blackbox_gpu_passes::{RadialBlur, RadialBlurIo, RadialBlurSettings};

use crate::ops::BlackboxBridge;
use crate::pipeline_cache::PipelineCacheFile;

/// The pass, created the first time a frame asks for the blur.
#[derive(Resource, Default)]
pub struct RadialBlurRender(Option<RadialBlur>);

/// The part of an output-sized texture the scene fills, on each axis, when drawn at `render_scale`.
fn filled_part(out_size: (u32, u32), render_scale: f32) -> [f32; 2] {
    let (width, height) = blackbox_gfx::scaled_size(out_size, render_scale);
    [width as f32 / out_size.0.max(1) as f32, height as f32 / out_size.1.max(1) as f32]
}

pub fn radial_blur_pass(
    mut render: ResMut<RadialBlurRender>,
    bridge: Res<BlackboxBridge>,
    pipeline_cache: Res<PipelineCacheFile>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    view: ViewQuery<(&ExtractedCamera, &ViewTarget)>,
    mut ctx: RenderContext,
) {
    let (focus, render_scale) = {
        let shared = bridge.lock();
        (shared.focus.sanitized(), shared.settings.render_scale)
    };
    if focus.radial_blur <= 0.0 {
        return;
    }
    let (camera, target) = view.into_inner();
    let Some(size) = camera.physical_target_size else { return };
    let device = device.wgpu_device();
    let pass =
        render.0.get_or_insert_with(|| RadialBlur::new(device).with_pipeline_cache(pipeline_cache.cache().cloned()));
    pass.set(RadialBlurSettings { strength: focus.radial_blur, centre: focus.radial_centre, ..pass.settings() });
    pass.set_content_scale(filled_part((size.x, size.y), render_scale));
    // The write swaps the view's two main textures: what the pass reads is the finished frame, what it writes becomes it.
    let write = target.post_process_write();
    let io =
        RadialBlurIo { input: write.source, output: write.destination, output_format: target.main_texture_format() };
    pass.encode(device, &queue, ctx.command_encoder(), &io);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scene_at_a_render_scale_fills_that_part_of_the_output_sized_texture() {
        assert_eq!(filled_part((1920, 1080), 1.0), [1.0, 1.0]);
        let part = filled_part((1920, 1080), 0.5);
        assert!((part[0] - 0.5).abs() < 0.01 && (part[1] - 0.5).abs() < 0.01, "{part:?}");
    }
}
