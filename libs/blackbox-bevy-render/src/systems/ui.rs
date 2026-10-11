//! The UI layer: menus, HUD and the console, drawn through `blackbox_gpu_passes::UiPass` strictly after
//! `upscaling` (`bevy_core_pipeline::upscaling::upscaling`), at the output's own resolution, unscaled and
//! un-post-processed — matching native's `encode_ui`, which runs after the scene and after the whole
//! post chain, straight onto the surface.

use bevy_ecs::resource::Resource;
use bevy_ecs::system::{Res, ResMut};
use bevy_render::camera::ExtractedCamera;
use bevy_render::renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery};
use bevy_render::view::ViewTarget;
use blackbox_gfx::UiTexturePatch;
use blackbox_gpu_passes::UiPass;

use crate::ops::{BlackboxBridge, UiOp};

/// Owns the pipeline, the UI textures and the reused vertex and index buffers, created once the device
/// exists (the first frame a view runs after `upscaling`).
#[derive(Resource, Default)]
pub struct UiRender(Option<UiPass>);

pub fn ui_pass(
    mut render: ResMut<UiRender>,
    bridge: Res<BlackboxBridge>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    view: ViewQuery<(&ExtractedCamera, &ViewTarget)>,
    mut ctx: RenderContext,
) {
    let (camera, target) = view.into_inner();
    let Some(out_view) = target.out_texture() else { return };
    let Some(size) = camera.physical_target_size else { return };
    let format = target.out_texture_view_format().unwrap_or_else(|| target.main_texture_format());

    let pass = render.0.get_or_insert_with(|| UiPass::new(device.wgpu_device()));

    let (ops, layer) = {
        let mut shared = bridge.lock();
        (std::mem::take(&mut shared.ui_ops), shared.ui_layer.clone())
    };
    for op in ops {
        match op {
            UiOp::Update { id, offset, size, rgba } => {
                let patch = UiTexturePatch { id, offset, size, rgba: &rgba };
                if let Err(e) = pass.update_texture(device.wgpu_device(), &queue, &patch) {
                    log::warn!("bevy renderer: {e}");
                }
            }
            UiOp::Free(id) => pass.free_texture(id),
        }
    }
    pass.set_layer(layer);

    let encoder = ctx.command_encoder();
    pass.encode(device.wgpu_device(), &queue, encoder, out_view, format, (size.x, size.y));
}
