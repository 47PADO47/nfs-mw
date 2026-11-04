//! Native GPU snapshots for real installed menu routes. Captures are opt-in and remain outside the checkout.

use crate::ui::{
    UiAssets,
    present::{BlackboxPresenter, Screen},
};
use blackbox_feng::UiTree;
use blackbox_gfx::{FrameParams, Projection, RgbaImage};
use blackbox_render::{Renderer, RendererOptions};
use glam::{Mat4, Vec3};

pub(in crate::frontend) fn render(tree: &UiTree, assets: &UiAssets, size: (u32, u32), name: &str) -> RgbaImage {
    let mut renderer = Renderer::headless(
        size,
        RendererOptions {
            backend: blackbox_gfx::GraphicsApi::Vulkan,
            vsync: false,
            force_fallback_adapter: false,
            pipeline_cache_dir: None,
        },
    )
    .expect("Vulkan adapter required; absence is not a pass");
    let mut out = crate::gui::UiOutput::default();
    BlackboxPresenter::default().present(
        tree,
        assets,
        Screen { width: size.0 as f32, height: size.1 as f32, pixels_per_point: 1.0 },
        &mut out,
    );
    for patch in &out.patches {
        renderer.update_ui_texture(&patch.as_patch());
    }
    renderer.set_ui_layer(out.layer.take().unwrap());
    let frame = FrameParams {
        view: Mat4::IDENTITY,
        projection: Projection::Identity,
        camera_position: Vec3::ZERO,
        light_dir: Vec3::NEG_Z,
        clear_color: [0.2, 0.4, 0.6],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: true,
        shadow_tuning: None,
    };
    renderer.render(&frame, &[]).unwrap();
    let image = renderer.read_output().unwrap().unwrap();
    if let Some(folder) = std::env::var_os("NFSMW_UI_CAPTURE_DIR") {
        let path = std::path::PathBuf::from(folder).join(format!("{name}-{}x{}.png", size.0, size.1));
        let file = std::fs::File::create(path).unwrap();
        let mut encoder = png::Encoder::new(file, size.0, size.1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header().unwrap().write_image_data(&image.rgba).unwrap();
    }
    image
}
