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
