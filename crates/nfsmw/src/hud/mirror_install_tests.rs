//! Installed bezel filtering must never reveal the world through the glass edge.
use crate::gui::UiOutput;
use crate::ui::{
    Catalog, UiAssets,
    present::{BlackboxPresenter, Screen, blackbox::MIRROR_TEXTURE},
};
use blackbox_feng::NodeKind;
use blackbox_gfx::{FrameParams, GraphicsApi, Projection, UiTexturePatch};
use game_install::GameDir;
use glam::{Mat4, Vec3};

// Independent image-space reference: the transparent component enclosed by opaque frame pixels.
fn glass(rgba: &[u8], w: usize, h: usize) -> Vec<bool> {
    let mut runs = Vec::new();
    let mut first = None;
    for y in 0..=h {
        if y < h && rgba[y * w * 4 + 3] < 255 {
            first.get_or_insert(y);
            continue;
        }
        let Some(start) = first.take() else { continue };
        runs.push(start..y);
    }
    let run = runs.into_iter().max_by_key(|r| r.len()).expect("glass opening");
    let mut seen = vec![false; w * h];
    let mut pending = vec![((run.start + run.end) / 2) * w];
    while let Some(i) = pending.pop() {
        if seen[i] || rgba[i * 4 + 3] == 255 {
            continue;
        }
        seen[i] = true;
        let (x, y) = (i % w, i / w);
        if x > 0 {
            pending.push(i - 1);
        }
        if x + 1 < w {
            pending.push(i + 1);
        }
        if y > 0 {
            pending.push(i - w);
        }
        if y + 1 < h {
            pending.push(i + w);
        }
    }
    seen
}

#[test]
#[ignore = "needs installed HUD artwork and a native GPU; runs without a window"]
fn installed_mirror_filtered_edges_cover_the_world_at_1080p_and_4k() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR")).unwrap();
    let assets = UiAssets::load(&dir).unwrap();
    let catalog = Catalog::load(&dir, &["GLOBAL/InGameB.bun", "GLOBAL/INGAMEC.BUN"]);
    let mut rt = crate::ui::runtime();
    let id = rt.load(catalog.find("HUD_SingleRace.fng").unwrap().clone());
    let mut binding = super::bind::HudBinding::new(&mut rt, id);
    binding.advance_mirror_fade(1.0);
    binding.apply(&mut rt, &super::HudState::default());
    rt.update(1.0);
    binding.finish_frame(&mut rt, &super::HudState::default());
    let tree = rt.tree(id);
    for (width, height) in [(1920, 1080), (3840, 2160)] {
        let mut r = blackbox_render::Renderer::headless(
            (width, height),
            blackbox_render::RendererOptions {
                backend: GraphicsApi::Vulkan,
                vsync: false,
                force_fallback_adapter: false,
                pipeline_cache_dir: None,
            },
        )
        .expect("Vulkan adapter");
        let mut out = UiOutput::default();
        BlackboxPresenter::default().present(
            &tree,
            &assets,
            Screen { width: width as f32, height: height as f32, pixels_per_point: 1.0 },
            &mut out,
        );
        for patch in &out.patches {
            r.update_ui_texture(&patch.as_patch());
        }
        // Cyan camera, magenta world: red pixels in the glass fringe prove a background leak.
        r.update_ui_texture(&UiTexturePatch {
            id: MIRROR_TEXTURE,
            offset: None,
            size: [1, 1],
            rgba: &[0, 255, 255, 255],
        });
        r.set_ui_layer(out.layer.take().unwrap());
        r.render(
            &FrameParams {
                view: Mat4::IDENTITY,
                projection: Projection::Identity,
                camera_position: Vec3::ZERO,
                light_dir: Vec3::NEG_Z,
                clear_color: [1.0, 0.0, 1.0],
                fog: None,
                scattering: None,
                surface_light: None,
                camera_cut: true,
                shadow_tuning: None,
            },
            &[],
        )
        .unwrap();
        let image = r.read_output().unwrap().unwrap();
        let mut checked = 0;
        let mut leaks = 0;
        for node in tree.draw_order.iter().map(|&i| &tree.nodes[i]).filter(|n| n.name_hash == 0x5630_E0B9) {
            let NodeKind::Image { texture, uv, .. } = node.kind else { continue };
            let art = assets.image(texture).unwrap();
            let (w, h) = (art.width as usize, art.height as usize);
            let component = glass(&art.rgba, w, h);
            // FEng can collapse Z for flat sprites, so its 4x4 transform need not be invertible.
            let inverse =
                glam::Mat2::from_cols(node.world.x_axis.truncate().truncate(), node.world.y_axis.truncate().truncate())
                    .inverse();
            let translation = node.world.w_axis.truncate().truncate();
            let scale = height as f32 / 480.0;
            for y in 0..height / 3 {
                for x in width / 4..width * 3 / 4 {
                    let local = inverse
                        * (glam::Vec2::new(
                            (x as f32 + 0.5 - width as f32 * 0.5) / scale,
                            (y as f32 + 0.5 - height as f32 * 0.5) / scale,
                        ) - translation);
                    let u = uv[0] + (local.x + 0.5) * (uv[2] - uv[0]);
                    let v = uv[1] + (local.y + 0.5) * (uv[3] - uv[1]);
                    if !(0.0..1.0).contains(&u) || !(0.0..1.0).contains(&v) {
                        continue;
                    }
                    let (px, py) = (u * w as f32 - 0.5, v * h as f32 - 0.5);
                    let (fx, fy) = (px - px.floor(), py - py.floor());
                    let mut alpha = 0.0;
                    let mut belongs = false;
                    for dy in 0..=1 {
                        for dx in 0..=1 {
                            let sx = (px.floor() as i32 + dx).clamp(0, w as i32 - 1) as usize;
                            let sy = (py.floor() as i32 + dy).clamp(0, h as i32 - 1) as usize;
                            let weight = if dx == 0 { 1.0 - fx } else { fx } * if dy == 0 { 1.0 - fy } else { fy };
                            alpha += art.rgba[(sy * w + sx) * 4 + 3] as f32 / 255.0 * weight;
                            belongs |= weight > 0.001 && component[sy * w + sx];
                        }
                    }
                    if belongs && (0.05..0.95).contains(&alpha) {
                        checked += 1;
                        leaks += usize::from(image.pixel(x, y).unwrap()[0] > 8);
                    }
                }
            }
        }
        if let Some(folder) = std::env::var_os("NFSMW_MIRROR_CAPTURE_DIR") {
            let file =
                std::fs::File::create(std::path::PathBuf::from(folder).join(format!("mirror-{width}.png"))).unwrap();
            let mut encoder = png::Encoder::new(file, width, height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header().unwrap().write_image_data(&image.rgba).unwrap();
        }
        assert!(checked > 100, "must sample actual bilinear glass-edge pixels, got {checked}");
        assert_eq!(leaks, 0, "{width}x{height}: {leaks}/{checked} glass-edge pixels expose the world");
        eprintln!("{width}x{height}: {checked} filtered edge pixels, {leaks} leaks");
    }
}
