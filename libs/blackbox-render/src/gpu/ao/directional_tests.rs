//! Directional AO uses the same visibility mask and keeps direct light independent.
use super::tests::scene_shading;
use crate::gpu::test_support;
use blackbox_gfx::{AoMethod, AoQuality, AoSettings, GraphicsSettings, RenderBackend, RenderExperiments, Shading};
use glam::Vec3;

#[test]
#[ignore = "needs a GPU"]
fn bent_normal_ao_steers_away_from_a_blocker_and_preserves_open_sky() {
    let _serial = test_support::serial();
    let size = (319, 237);
    let mut r = test_support::headless(size).expect("GPU adapter required");
    let (frame, blocked) = scene_shading(&mut r, size, true, Shading::Lit);
    let (_, open) = scene_shading(&mut r, size, false, Shading::Lit);
    let settings = GraphicsSettings {
        ao: AoSettings { method: AoMethod::XeGtao, quality: AoQuality::High, intensity: 1.0, ..Default::default() },
        ..Default::default()
    };
    let lighting = RenderExperiments { world_skylight: true, world_skylight_strength: 1.0, ..Default::default() };
    r.set_render_experiments(lighting);
    r.apply_graphics(&settings);
    let scalar_image = r.capture(size.0, size.1, &frame, &blocked).unwrap();
    let scalar_mask = r.read_raw_texture(r.shared.shadows.ambient.texture(), size, 1).unwrap();

    let directional = GraphicsSettings { ao: AoSettings { bent_normals: true, ..settings.ao }, ..settings };
    r.apply_graphics(&directional);
    let bent_image = r.capture(size.0, size.1, &frame, &blocked).unwrap();
    let mask = r.read_raw_texture(r.shared.shadows.ambient.texture(), size, 4).unwrap();
    let pixels = mask.as_chunks::<4>().0;
    let delta = pixels.iter().zip(&scalar_mask).map(|(p, s)| u64::from(p[0].abs_diff(*s))).sum::<u64>();
    assert!(delta < pixels.len() as u64 / 3, "directional resolve keeps scalar AO within packing tolerance: {delta}");
    assert_ne!(scalar_image, bent_image, "the world ambient receiver must use the direction");
    let clip = frame.projection.matrix() * frame.view;
    let mut samples = 0;
    let mut away = 0;
    for y in -8..8 {
        for x in 0..10 {
            let ground = Vec3::new(0.76 + x as f32 * 0.025, y as f32 * 0.04, 0.0);
            let point = clip.project_point3(ground);
            let x = ((0.5 + 0.5 * point.x) * size.0 as f32) as usize;
            let y = ((0.5 - 0.5 * point.y) * size.1 as f32) as usize;
            let [_, oct_x, _, strength] = pixels[y * size.0 as usize + x];
            if strength > 20 {
                samples += 1;
                away += usize::from(oct_x > 132);
            }
        }
    }
    assert!(samples > 30 && away * 5 > samples * 4, "the east side bends east: {away}/{samples}");
    let bent_open = r.capture(size.0, size.1, &frame, &open).unwrap();
    let open_mask = r.read_raw_texture(r.shared.shadows.ambient.texture(), size, 4).unwrap();
    assert!(open_mask.as_chunks::<4>().0.iter().all(|p| p[3] <= 1), "open geometry has no directional strength");
    r.apply_graphics(&settings);
    assert_eq!(bent_open, r.capture(size.0, size.1, &frame, &open).unwrap());
    assert_eq!(scalar_image, r.capture(size.0, size.1, &frame, &blocked).unwrap(), "toggle restores the scalar path");

    r.apply_graphics(&GraphicsSettings { ao: AoSettings { compare: true, ..directional.ao }, ..directional });
    r.capture(size.0, size.1, &frame, &blocked).unwrap();
    let mask = r.read_raw_texture(r.shared.shadows.ambient.texture(), size, 4).unwrap();
    for row in mask.chunks_exact(size.0 as usize * 4) {
        assert!(row.as_chunks::<4>().0[..(size.0 / 2) as usize].iter().all(|p| p[0] == 255 && p[3] == 0));
    }
}

#[test]
#[ignore = "needs a GPU"]
fn temporal_bent_normal_ao_clears_disoccluded_and_reset_history() {
    let _serial = test_support::serial();
    let size = (319, 237);
    let mut r = test_support::headless(size).expect("GPU adapter required");
    let (mut frame, blocked) = scene_shading(&mut r, size, true, Shading::Lit);
    let (_, open) = scene_shading(&mut r, size, false, Shading::Lit);
    r.set_render_experiments(RenderExperiments { world_skylight: true, ..Default::default() });
    r.apply_graphics(&GraphicsSettings {
        ao: AoSettings {
            method: AoMethod::TemporalXeGtao,
            bent_normals: true,
            quality: AoQuality::High,
            ..Default::default()
        },
        ..Default::default()
    });
    let cold = r.capture(size.0, size.1, &frame, &blocked).unwrap();
    for _ in 0..8 {
        r.capture(size.0, size.1, &frame, &blocked).unwrap();
    }
    let mask = r.read_raw_texture(r.shared.shadows.ambient.texture(), size, 4).unwrap();
    assert!(mask.as_chunks::<4>().0.iter().any(|p| p[3] > 60));
    r.capture(size.0, size.1, &frame, &open).unwrap();
    let mask = r.read_raw_texture(r.shared.shadows.ambient.texture(), size, 4).unwrap();
    assert!(mask.as_chunks::<4>().0.iter().all(|p| p[3] <= 1), "old direction clears after disocclusion");
    frame.camera_cut = true;
    assert_eq!(cold, r.capture(size.0, size.1, &frame, &blocked).unwrap());
}
