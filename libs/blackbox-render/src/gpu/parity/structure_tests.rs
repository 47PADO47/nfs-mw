//! Facts about the testkit pictures that hold on any adapter: what is in front of what, where fog ends,
//! what a clip rectangle hides. They do not depend on exact colours, so they run on lavapipe too.

use std::sync::MutexGuard;

use blackbox_gfx::{Antialiasing, GraphicsSettings, PostSettings, RenderBackend, RgbaImage, Tonemap, Upscaler};
use blackbox_gfx_testkit::scenes::depth_probe::{DISTANCES, ROW_BEHIND, ROW_CROSSING, column_x};
use blackbox_gfx_testkit::{Mask, SceneId, compare};
use glam::Vec3;

use super::{SIZE, capture, project, rgb};
use crate::Renderer;
use crate::gpu::test_support;

/// A headless renderer with default settings (the scene goes straight into the output).
fn setup() -> Option<(MutexGuard<'static, ()>, Renderer)> {
    let guard = test_support::serial();
    let renderer = test_support::headless((SIZE[0], SIZE[1]))?;
    Some((guard, renderer))
}

fn shot(renderer: &mut Renderer, scene: SceneId) -> (RgbaImage, blackbox_gfx::FrameParams) {
    capture(renderer, scene).expect("capture")
}

fn is_red(c: [u8; 3]) -> bool {
    c[0] > 150 && c[1] < 100 && c[2] < 100
}

fn is_green(c: [u8; 3]) -> bool {
    c[1] > 150 && c[0] < 100 && c[2] < 120
}

/// Whether channel `by` of `c` is more than `margin` above both others.
fn dominant(c: [u8; 3], by: usize, margin: i32) -> bool {
    (0..3).filter(|&i| i != by).all(|i| i32::from(c[by]) > i32::from(c[i]) + margin)
}

fn near(a: [u8; 3], b: [u8; 3], by: u8) -> bool {
    (0..3).all(|i| a[i].abs_diff(b[i]) <= by)
}

#[test]
#[ignore = "needs a GPU"]
fn every_scene_draws_something() {
    let Some((_gpu, mut renderer)) = setup() else { return };
    for scene in SceneId::ALL {
        let (image, _) = shot(&mut renderer, scene);
        let first = rgb(&image, (0, 0));
        let varied =
            (0..SIZE[1]).step_by(4).any(|y| (0..SIZE[0]).step_by(4).any(|x| !near(rgb(&image, (x, y)), first, 12)));
        assert!(varied, "{}: a blank picture", scene.name());
        assert_eq!(image.pixel(10, 10).map(|p| p[3]), Some(255), "{}: opaque output", scene.name());
    }
}

#[test]
#[ignore = "needs a GPU"]
fn captures_are_repeatable() {
    let Some((_gpu, mut renderer)) = setup() else { return };
    for scene in SceneId::ALL {
        let (a, _) = shot(&mut renderer, scene);
        let (b, _) = shot(&mut renderer, scene);
        assert_eq!(a, b, "{}: the same scene twice gives the same pixels", scene.name());
    }
}

#[test]
#[ignore = "needs a GPU"]
fn the_grid_fades_into_the_fog_colour() {
    let Some((_gpu, mut renderer)) = setup() else { return };
    let (image, frame) = shot(&mut renderer, SceneId::Grid);
    let clear = frame.clear_color.map(|c| (c * 255.0).round() as u8);
    let far = rgb(&image, project(&frame, Vec3::new(-8.0, 50.0, 0.0)));
    assert!(near(far, clear, 4), "fully fogged ground is the fog colour: {far:?} vs {clear:?}");
    let close = rgb(&image, project(&frame, Vec3::new(0.0, 2.0, 0.0)));
    assert!(!near(close, clear, 30), "ground inside the fog start keeps its own colour: {close:?}");
}

#[test]
#[ignore = "needs a GPU"]
fn alpha_tested_cards_cut_out_their_corners() {
    let Some((_gpu, mut renderer)) = setup() else { return };
    let (image, frame) = shot(&mut renderer, SceneId::AlphaCards);
    // Card 3 of the front row: centre (-1.5, 6 + sin 2.7, 0.2), 3.4 square, turned -0.06.
    let centre = Vec3::new(-1.5, 6.0 + 2.7_f32.sin(), 0.2);
    let along = Vec3::new((-0.06_f32).cos(), (-0.06_f32).sin(), 0.0);
    let at = |u: f32, v: f32| rgb(&image, project(&frame, centre + along * (u * 1.7) + Vec3::Z * (1.7 + v * 1.7)));
    let leaf = at(0.4, 0.0);
    assert!(dominant(leaf, 1, 50), "a green leaf: {leaf:?}");
    let corner = at(0.9, 0.9);
    assert!(!dominant(corner, 1, 50), "the corner is cut out: {corner:?}");
}

#[test]
#[ignore = "needs a GPU"]
fn later_blends_go_on_top_and_depth_hides_what_is_behind_the_wall() {
    let Some((_gpu, mut renderer)) = setup() else { return };
    let (image, frame) = shot(&mut renderer, SceneId::BlendStack);
    // Red then green overlap here: the green, drawn second, ends on top.
    let both = rgb(&image, project(&frame, Vec3::new(-3.7, 8.0, 4.8)));
    assert!(i32::from(both[1]) > i32::from(both[0]) + 20, "green over red: {both:?}");
    // An additive white quad sits behind the wall and must not show.
    let hidden = rgb(&image, project(&frame, Vec3::new(3.0, 10.0, 6.0)));
    assert!(hidden.iter().all(|&c| c < 235), "the wall hides the additive quad: {hidden:?}");
}

#[test]
#[ignore = "needs a GPU"]
fn the_far_dome_is_sky_behind_everything() {
    let Some((_gpu, mut renderer)) = setup() else { return };
    let (image, frame) = shot(&mut renderer, SceneId::SkyDome);
    let top = rgb(&image, (SIZE[0] / 2, 2));
    assert!(dominant(top, 2, 60), "blue sky overhead: {top:?}");
    let ground = rgb(&image, (SIZE[0] / 2, SIZE[1] - 3));
    assert!(dominant(ground, 1, 20), "green ground below: {ground:?}");
    let building = rgb(&image, project(&frame, Vec3::new(-30.0, 53.0, 10.0)));
    let spread = building.iter().max().unwrap() - building.iter().min().unwrap();
    assert!(spread < 30 && building[0] > 100, "a grey building in front of the sky: {building:?}");
}

#[test]
#[ignore = "needs a GPU"]
fn reverse_z_keeps_its_precision_out_to_nine_kilometres() {
    let Some((_gpu, mut renderer)) = setup() else { return };
    let (image, frame) = shot(&mut renderer, SceneId::DepthProbe);
    for (k, scale) in DISTANCES.into_iter().enumerate() {
        let x = column_x(k);
        let at = |x: f32, y: f32, z: f32| rgb(&image, project(&frame, Vec3::new(x, y, z) * scale));
        // Crossing quads: the green one is in front on the left, behind on the right.
        let left = at(x - 0.036, 0.92, ROW_CROSSING);
        let right = at(x + 0.10, 1.0, ROW_CROSSING);
        assert!(is_green(left), "{scale} m, crossing, left of the line: {left:?}");
        assert!(is_red(right), "{scale} m, crossing, right of the line: {right:?}");
        // 0.1 % behind: red wins where they overlap, green shows only beside it.
        let overlap = at(x + 0.05, 1.0, ROW_BEHIND);
        let beside = at(x + 0.16, 1.001, ROW_BEHIND);
        assert!(is_red(overlap), "{scale} m, 0.1 % apart, overlap: {overlap:?}");
        assert!(is_green(beside), "{scale} m, 0.1 % apart, beside: {beside:?}");
    }
}

#[test]
#[ignore = "needs a GPU"]
fn effects_land_where_they_are_placed() {
    let Some((_gpu, mut renderer)) = setup() else { return };
    let (image, frame) = shot(&mut renderer, SceneId::Effects);
    let glow = rgb(&image, project(&frame, Vec3::new(-5.0, 12.0, 3.2)));
    assert!(glow[0] > 200 && glow[0] > glow[2], "an orange glow: {glow:?}");
    let mark = rgb(&image, project(&frame, Vec3::new(-1.85, 6.0, 0.02)));
    let floor = rgb(&image, project(&frame, Vec3::new(-4.0, 6.0, 0.0)));
    let luma = |c: [u8; 3]| u32::from(c[0]) + u32::from(c[1]) + u32::from(c[2]);
    assert!(luma(mark) + 60 < luma(floor), "a dark mark on the floor: {mark:?} vs {floor:?}");
}

#[test]
#[ignore = "needs a GPU"]
fn the_ui_layer_clips_blends_and_textures() {
    let Some((_gpu, mut renderer)) = setup() else { return };
    let (image, _) = shot(&mut renderer, SceneId::Ui);
    let scale = SIZE[0] as f32 / 320.0;
    let px = |x: f32, y: f32| rgb(&image, ((x * scale) as u32, (y * scale) as u32));
    assert!(near(px(30.0, 20.0), [30, 60, 160], 2), "an opaque panel: {:?}", px(30.0, 20.0));
    let overlay = px(180.0, 100.0);
    assert!(dominant(overlay, 0, 40), "a translucent red overlay: {overlay:?}");
    let green = px(200.0, 130.0);
    assert!(near(green, [30, 160, 70], 2), "inside the clip rectangle: {green:?}");
    let clipped = px(120.0, 150.0);
    assert!(!near(clipped, [30, 160, 70], 40), "outside the clip rectangle: {clipped:?}");
    let patch = px(243.0, 28.0);
    assert!(near(patch, [220, 30, 30], 4), "the partly updated texture region: {patch:?}");
    let cell = px(218.0, 14.0);
    assert!(cell.iter().all(|&c| c > 225), "a light checker cell: {cell:?}");
    let triangle = px(60.0, 150.0);
    assert!(triangle.iter().all(|&c| (60..110).contains(&c)), "the vertex colours blend: {triangle:?}");
    let backend: &dyn RenderBackend = &renderer;
    assert_eq!(backend.surface_size(), SIZE);
}

#[test]
#[ignore = "needs a GPU"]
fn fxaa_smooths_edges_and_leaves_flat_areas_alone() {
    let Some((_gpu, mut renderer)) = setup() else { return };
    let (plain, _) = shot(&mut renderer, SceneId::DepthProbe);
    let fxaa = GraphicsSettings {
        post: PostSettings { antialiasing: Antialiasing::Fxaa, ..PostSettings::default() },
        ..GraphicsSettings::default()
    };
    assert!(renderer.apply_graphics(&fxaa).is_exact());
    let (smoothed, _) = shot(&mut renderer, SceneId::DepthProbe);
    let diff = compare(&plain, &smoothed, &Mask::all()).expect("same size");
    assert!(diff.max >= 20, "edges between the saturated quads and the dark background change: {diff}");
    assert!(diff.mean < 2.0, "most of the picture is flat and untouched: {diff}");
    assert_eq!(rgb(&plain, (SIZE[0] / 2, 5)), rgb(&smoothed, (SIZE[0] / 2, 5)), "flat background");
}

#[test]
#[ignore = "needs a GPU"]
fn the_ui_layer_is_never_post_processed_or_scaled() {
    let Some((_gpu, mut renderer)) = setup() else { return };
    let panel = |renderer: &mut Renderer| {
        let (image, _) = shot(renderer, SceneId::Ui);
        let scale = SIZE[0] as f32 / 320.0;
        rgb(&image, ((30.0 * scale) as u32, (20.0 * scale) as u32))
    };
    let direct = panel(&mut renderer);
    let heavy = GraphicsSettings {
        post: PostSettings {
            tonemap: Tonemap::Aces,
            bloom_intensity: 1.0,
            bloom_threshold: 0.2,
            antialiasing: Antialiasing::Fxaa,
            ..PostSettings::default()
        },
        upscaler: Upscaler::Fsr1,
        render_scale: 0.5,
        ..GraphicsSettings::default()
    };
    assert!(renderer.apply_graphics(&heavy).is_exact());
    assert!(!renderer.draws_directly());
    assert_eq!(panel(&mut renderer), direct, "the panel keeps its colour under every pass");
    assert!(near(direct, [30, 60, 160], 2));
}
