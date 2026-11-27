//! A dynamic model in front of a static crease keeps the crease's XeGTAO occlusion: the horizon search also reads
//! the static depth the model hides.

use blackbox_gfx::{
    AoMethod, AoQuality, AoSettings, BlendMode, FrameParams, GraphicsSettings, Instance, Projection, RenderBackend,
    Shading,
};
use blackbox_gfx_testkit::mesh::{MeshBuilder, prelit};
use glam::{Mat4, Vec3};

use crate::{Renderer, gpu::test_support};

const SIZE: (u32, u32) = (480, 320);

/// A sidewalk meeting a wall along y = 1, seen from above and in front.
fn crease(renderer: &mut Renderer) -> (FrameParams, Instance) {
    let mut mesh = MeshBuilder::new();
    mesh.state(None, BlendMode::Opaque, Shading::Prelit);
    let grey = prelit([0.65; 3], 1.0);
    mesh.flat_quad(
        [
            Vec3::new(-10.0, -10.0, 0.0),
            Vec3::new(10.0, -10.0, 0.0),
            Vec3::new(10.0, 1.0, 0.0),
            Vec3::new(-10.0, 1.0, 0.0),
        ],
        grey,
    );
    mesh.flat_quad(
        [Vec3::new(-10.0, 1.0, 0.0), Vec3::new(10.0, 1.0, 0.0), Vec3::new(10.0, 1.0, 3.0), Vec3::new(-10.0, 1.0, 3.0)],
        grey,
    );
    let handle = mesh.finish().upload(renderer, "AO crease");
    let eye = Vec3::new(1.0, -3.2, 2.8);
    let frame = FrameParams {
        view: glam::camera::rh::view::look_at_mat4(eye, Vec3::new(0.0, 1.0, 0.2), Vec3::Z),
        projection: Projection::PerspectiveInfiniteReverse {
            fov_y: 1.0,
            aspect: SIZE.0 as f32 / SIZE.1 as f32,
            near: 0.1,
        },
        camera_position: eye,
        light_dir: Vec3::NEG_Z,
        clear_color: [0.2, 0.3, 0.4],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: false,
        shadow_tuning: None,
    };
    (frame, Instance::new(handle, Mat4::IDENTITY))
}

/// A car-sized body parked against the wall, raised off the ground. `Lit` is a vehicle material; `Prelit` makes the
/// same body part of the static scene.
fn body(renderer: &mut Renderer, shading: Shading) -> Instance {
    let mut mesh = MeshBuilder::new();
    mesh.state(None, BlendMode::Opaque, shading);
    mesh.cuboid(Vec3::new(-1.0, -1.2, 0.15), Vec3::new(1.0, 0.3, 0.7), prelit([0.5; 3], 1.0));
    Instance::new(mesh.finish().upload(renderer, "AO parked body"), Mat4::IDENTITY)
}

#[test]
#[ignore = "needs a GPU"]
fn parked_body_keeps_the_crease_occlusion_it_does_not_cover() {
    let _serial = test_support::serial();
    let Some(mut r) = test_support::headless(SIZE) else { return };
    let (frame, world) = crease(&mut r);
    let car = body(&mut r, Shading::Lit);
    let scenery = body(&mut r, Shading::Prelit);
    for (method, depth_mips) in
        [AoMethod::XeGtao, AoMethod::TemporalXeGtao].into_iter().flat_map(|m| [false, true].map(|d| (m, d)))
    {
        r.apply_graphics(&GraphicsSettings {
            ao: AoSettings { method, quality: AoQuality::High, depth_mips, intensity: 1.0, ..Default::default() },
            ..Default::default()
        });
        let mut frames = |instances: &[Instance]| {
            let mut cut = frame;
            cut.camera_cut = true;
            let colour = r.capture(SIZE.0, SIZE.1, &cut, instances).unwrap();
            let mask = r.read_raw_texture(r.shared.shadows.ambient.texture(), SIZE, 1).unwrap();
            (colour, mask)
        };
        // The body's own pixels: where drawing it alone changes the empty frame.
        let (empty, _) = frames(&[]);
        let (alone, _) = frames(&[car]);
        let (_, open) = frames(&[world]);
        let (_, dynamic) = frames(&[world, car]);
        let (_, fixed) = frames(&[world, scenery]);
        let mut lost_before = 0;
        let mut lost_after = 0;
        let mut darker = 0;
        for (i, ((&open, &dynamic), &fixed)) in open.iter().zip(&dynamic).zip(&fixed).enumerate() {
            assert!(
                dynamic <= fixed.saturating_add(1),
                "{method} mips={depth_mips}: the static depth may only add occlusion ({dynamic} > {fixed} at {i})"
            );
            darker += usize::from(dynamic.saturating_add(2) < fixed);
            let covered = alone[i * 4..i * 4 + 4] != empty[i * 4..i * 4 + 4];
            if covered || open > 200 {
                continue;
            }
            lost_before += usize::from(fixed > open.saturating_add(25));
            lost_after += usize::from(dynamic > open.saturating_add(25));
        }
        assert!(lost_before > 100, "{method} mips={depth_mips}: the scene must show the lost crease ({lost_before})");
        assert!(darker > 100, "{method} mips={depth_mips}: the hidden crease must darken ({darker})");
        assert!(
            lost_after * 4 < lost_before,
            "{method} mips={depth_mips}: crease pixels still lose occlusion ({lost_after} of {lost_before})"
        );
    }
}

#[test]
#[ignore = "needs a GPU"]
fn static_scenes_are_unchanged_without_dynamic_models() {
    let _serial = test_support::serial();
    let Some(mut r) = test_support::headless(SIZE) else { return };
    let (frame, world) = crease(&mut r);
    let scenery = body(&mut r, Shading::Prelit);
    r.apply_graphics(&GraphicsSettings {
        ao: AoSettings { method: AoMethod::XeGtao, quality: AoQuality::High, intensity: 1.0, ..Default::default() },
        ..Default::default()
    });
    let image = r.capture(SIZE.0, SIZE.1, &frame, &[world, scenery]).unwrap();
    let surfaces = r.surfaces.as_ref().expect("AO keeps the surface prepass");
    let snapshot = surfaces.static_depth.as_ref().expect("XeGTAO takes static depth snapshots");
    assert!(snapshot.current().is_none(), "no dynamic model drawn, so no static depth is read");
    assert_eq!(image, r.capture(SIZE.0, SIZE.1, &frame, &[world, scenery]).unwrap());
}

#[test]
#[ignore = "needs a GPU"]
fn hidden_crease_varies_less_between_jittered_frames() {
    let _serial = test_support::serial();
    let Some(mut r) = test_support::headless(SIZE) else { return };
    let (frame, world) = crease(&mut r);
    let car = body(&mut r, Shading::Lit);
    let scenery = body(&mut r, Shading::Prelit);
    let settings = |upscaler| GraphicsSettings {
        upscaler,
        upscale_quality: blackbox_gfx::UpscaleQuality::Native,
        ao: AoSettings { method: AoMethod::XeGtao, quality: AoQuality::High, intensity: 1.0, ..Default::default() },
        ..Default::default()
    };
    r.apply_graphics(&settings(blackbox_gfx::Upscaler::Off));
    let empty = r.capture(SIZE.0, SIZE.1, &frame, &[]).unwrap();
    let alone = r.capture(SIZE.0, SIZE.1, &frame, &[car]).unwrap();
    r.capture(SIZE.0, SIZE.1, &frame, &[world]).unwrap();
    let open = r.read_raw_texture(r.shared.shadows.ambient.texture(), SIZE, 1).unwrap();
    // The crease and its surroundings near the body, outside the body itself.
    let near: Vec<usize> = (0..open.len())
        .filter(|&i| open[i] < 230 && alone[i * 4..i * 4 + 4] == empty[i * 4..i * 4 + 4])
        .filter(|&i| {
            let (x, y) = ((i % SIZE.0 as usize) as i32, (i / SIZE.0 as usize) as i32);
            (-12..=12).any(|d| {
                [(x + d, y), (x, y + d)].iter().any(|&(u, v)| {
                    if !(0..SIZE.0 as i32).contains(&u) || !(0..SIZE.1 as i32).contains(&v) {
                        return false;
                    }
                    let j = v as usize * SIZE.0 as usize + u as usize;
                    alone[j * 4..j * 4 + 4] != empty[j * 4..j * 4 + 4]
                })
            })
        })
        .collect();
    assert!(near.len() > 200, "the body must border the crease ({})", near.len());
    r.apply_graphics(&settings(blackbox_gfx::Upscaler::Fsr3));
    let mut variation = |instances: &[Instance]| {
        let mut masks = Vec::new();
        for _ in 0..16 {
            r.render(&frame, instances).unwrap();
            masks.push(r.read_raw_texture(r.shared.shadows.ambient.texture(), SIZE, 1).unwrap());
        }
        let steps = masks.windows(2).flat_map(|w| near.iter().map(move |&i| w[0][i].abs_diff(w[1][i]) as f64));
        let (sum, count) = steps.fold((0.0, 0usize), |(s, c), v| (s + v, c + 1));
        sum / count as f64
    };
    let before = variation(&[world, scenery]);
    let after = variation(&[world, car]);
    println!("frame-to-frame AO change beside the body: {before:.2}/255 as scenery, {after:.2}/255 as a vehicle");
    assert!(after <= before * 1.05, "the static depth must not add frame-to-frame noise ({after:.2} > {before:.2})");
}
