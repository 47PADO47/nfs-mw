//! History must reset cleanly and never leave occlusion on newly uncovered geometry.
use super::tests::scene_shading;
use crate::gpu::test_support;
use blackbox_gfx::{AoMethod, AoQuality, AoSettings, GraphicsSettings, RenderBackend, Shading};

#[test]
#[ignore = "needs a GPU"]
fn temporal_contacts_reset_disocclude_resize_and_switch_to_spatial() {
    let _serial = test_support::serial();
    let size = (319, 237);
    let mut r = test_support::headless(size).expect("GPU adapter required");
    let (mut frame, closed) = scene_shading(&mut r, size, true, Shading::Prelit);
    let (_, open) = scene_shading(&mut r, size, false, Shading::Prelit);
    let settings = GraphicsSettings {
        ao: AoSettings { method: AoMethod::TemporalXeGtao, quality: AoQuality::High, ..Default::default() },
        ..Default::default()
    };
    r.apply_graphics(&settings);
    let cold = r.capture(size.0, size.1, &frame, &closed).unwrap();
    for _ in 0..8 {
        r.capture(size.0, size.1, &frame, &closed).unwrap();
    }
    let ao = r.ao.as_ref().unwrap();
    assert_eq!(ao.images.as_ref().unwrap().size, (160, 119));
    let mask = r.read_raw_texture(r.shared.shadows.ambient.texture(), size, 1).unwrap();
    assert!(mask.iter().any(|&v| v < 240), "contacts remain visible after accumulation");
    r.capture(size.0, size.1, &frame, &open).unwrap();
    let mask = r.read_raw_texture(r.shared.shadows.ambient.texture(), size, 1).unwrap();
    assert!(mask.iter().all(|&v| v >= 254), "no old contact after an occluder disappears");
    frame.camera_cut = true;
    assert_eq!(cold, r.capture(size.0, size.1, &frame, &closed).unwrap(), "cut starts cold");
    frame.camera_cut = false;
    r.resize(127, 93);
    r.capture(127, 93, &frame, &closed).unwrap();
    assert_eq!(r.ao.as_ref().unwrap().images.as_ref().unwrap().size, (64, 47));
    r.apply_graphics(&GraphicsSettings { ao: AoSettings { method: AoMethod::XeGtao, ..settings.ao }, ..settings });
    let spatial = r.capture(127, 93, &frame, &closed).unwrap();
    assert!(r.effect_motion.is_none());
    assert!(r.ao.as_ref().unwrap().temporal.is_none());
    assert_eq!(spatial, r.capture(127, 93, &frame, &closed).unwrap());
    r.apply_graphics(&GraphicsSettings::default());
    r.capture(127, 93, &frame, &closed).unwrap();
    assert!(r.ao.is_none() && r.effect_motion.is_none());
}

#[test]
#[ignore = "needs a GPU"]
fn following_a_moving_receiver_keeps_temporal_ao_as_stable_as_a_static_receiver() {
    let _serial = test_support::serial();
    let size = (319, 237);
    let mut r = test_support::headless(size).expect("GPU adapter required");
    let (frame, mut instances) = scene_shading(&mut r, size, true, Shading::Prelit);
    instances[0].key = blackbox_gfx::InstanceKey::new(7, 1);
    r.apply_graphics(&GraphicsSettings {
        ao: AoSettings { method: AoMethod::TemporalXeGtao, quality: AoQuality::Medium, ..Default::default() },
        ..Default::default()
    });
    let forward = frame.view.inverse().transform_vector3(glam::Vec3::NEG_Z);
    let mut measure = |moving: bool| {
        let mut previous: Option<Vec<u8>> = None;
        let mut difference = 0.0;
        let mut samples = 0;
        for i in 0..48 {
            let distance = match moving {
                true => i as f32 * 0.3,
                false => 0.0,
            };
            let transform = glam::Mat4::from_translation(forward * distance);
            instances[0].transform = transform;
            let mut current = frame;
            current.view = frame.view * transform.inverse();
            current.camera_position = transform.transform_point3(frame.camera_position);
            current.camera_cut = i == 0;
            r.render(&current, &instances).unwrap();
            let mask = r.read_raw_texture(r.shared.shadows.ambient.texture(), size, 1).unwrap();
            if i >= 16
                && let Some(before) = &previous
            {
                for (&a, &b) in before.iter().zip(&mask) {
                    if a.min(b) >= 245 {
                        continue;
                    }
                    difference += f64::from(a.abs_diff(b));
                    samples += 1;
                }
            }
            previous = Some(mask);
        }
        assert!(samples > 1000, "test contains measurable contacts");
        difference / f64::from(samples)
    };
    let stationary = measure(false);
    let following = measure(true);
    println!("temporal AO mean frame delta: stationary={stationary:.4}, following={following:.4}");
    assert!(stationary < 1.5, "static temporal grain must be below 1.5 mask levels: {stationary:.4}");
    assert!(
        following <= stationary * 1.35 + 0.03,
        "following camera must not discard object history: stationary={stationary:.4}, following={following:.4}"
    );
}
