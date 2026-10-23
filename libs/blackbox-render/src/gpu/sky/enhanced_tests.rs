//! Actual raster checks: weather, moving volume and authored fallback.
use super::*;

#[test]
#[ignore = "needs a GPU"]
fn enhanced_horizon_matches_encoded_scene_fog_and_ignores_authored_cloud_overlays() {
    let _gpu = test_support::serial();
    let Some(mut renderer) = test_support::headless((32, 32)) else { return };
    let red = texture(&mut renderer, "authored cloud overlay", 1, &[255, 0, 0, 255]);
    let meshes = [SkyLayerKind::Base, SkyLayerKind::Cloud3].map(|kind| {
        let material =
            renderer.create_sky_material(&SkyMaterialDesc { kind, diffuse: red, miscellaneous: red }).unwrap();
        mesh(&mut renderer, material, [0.5; 2])
    });
    let mut view = frame();
    view.camera_position = Vec3::Z * 4.0;
    renderer.set_sky_state(SkyState { enhanced: true, ..Default::default() });
    for (fog, expected) in [([0.32, 0.45, 0.6], [82, 115, 153]), ([0.0; 3], [0; 3])] {
        view.clear_color = fog;
        renderer.render(&view, &meshes.map(|m| Instance::new(m, Mat4::IDENTITY))).unwrap();
        let actual = renderer.read_output().unwrap().unwrap().pixel(16, 16).unwrap();
        close(actual, expected);
    }
}

fn image(renderer: &mut Renderer, mesh: blackbox_gfx::MeshHandle, state: SkyState) -> Vec<[u8; 4]> {
    renderer.set_sky_state(state);
    renderer.render(&frame(), &[Instance::new(mesh, Mat4::IDENTITY)]).unwrap();
    let output = renderer.read_output().unwrap().unwrap();
    (0..32).flat_map(|y| (0..32).map(move |x| (x, y))).map(|(x, y)| output.pixel(x, y).unwrap()).collect()
}

#[test]
#[ignore = "needs a GPU"]
fn projected_vertical_mirror_preserves_the_sky_direction_despite_placement_offset() {
    let _gpu = test_support::serial();
    let Some(mut renderer) = test_support::headless((32, 32)) else { return };
    let source = texture(&mut renderer, "unused authored source", 1, &[255; 4]);
    let material = renderer
        .create_sky_material(&SkyMaterialDesc { kind: SkyLayerKind::Base, diffuse: source, miscellaneous: source })
        .unwrap();
    let mesh = mesh(&mut renderer, material, [0.5; 2]);
    let state = SkyState { enhanced: true, cloud_coverage: 0.5, ..Default::default() };
    let expected = image(&mut renderer, mesh, state);
    let mirrored =
        Mat4::from_scale_rotation_translation(Vec3::new(1.0, 1.0, -1.0), glam::Quat::IDENTITY, Vec3::Z * 130.0);
    let mut view = frame();
    view.view = mirrored.inverse();
    renderer.render(&view, &[Instance::new(mesh, mirrored)]).unwrap();
    let output = renderer.read_output().unwrap().unwrap();
    for y in 0..32 {
        for x in 0..32 {
            let actual = output.pixel(x, y).unwrap();
            let expected = expected[(y * 32 + x) as usize];
            for channel in 0..3 {
                assert!(actual[channel].abs_diff(expected[channel]) <= 2, "{actual:?} != {expected:?}");
            }
        }
    }
}

#[test]
#[ignore = "needs a GPU"]
fn enhanced_sky_weather_wind_and_authored_fallback_remain_independent() {
    let _gpu = test_support::serial();
    let Some(mut renderer) = test_support::headless((32, 32)) else { return };
    let source = texture(&mut renderer, "authored red", 1, &[255, 0, 0, 255]);
    let material = renderer
        .create_sky_material(&SkyMaterialDesc { kind: SkyLayerKind::Base, diffuse: source, miscellaneous: source })
        .unwrap();
    let mesh = mesh(&mut renderer, material, [0.5; 2]);
    let clear = SkyState { enhanced: true, ..Default::default() };
    let covered = SkyState { cloud_coverage: 1.0, ..clear };
    let a = image(&mut renderer, mesh, clear);
    let b = image(&mut renderer, mesh, covered);
    let c = image(&mut renderer, mesh, SkyState { scroll: 900.0, ..covered });
    assert!(a.iter().zip(&b).filter(|(a, b)| a != b).count() > 512, "weather must change the sky");
    assert!(b.iter().zip(&c).filter(|(a, b)| a != b).count() > 128, "wind must move volumetric density");
    assert!(a.iter().all(|p| p[3] == 255), "sky must remain opaque");
    assert!(a.iter().any(|p| p[2] > p[0]), "clear sky must contain blue scattering");
    let original = image(&mut renderer, mesh, SkyState::default());
    assert!(original.iter().all(|p| p[0] > 250 && p[1] < 3 && p[2] < 3));
    assert_eq!(a, image(&mut renderer, mesh, clear), "switching back must be deterministic");
}

#[test]
#[ignore = "needs a GPU"]
fn cloud_lighting_is_shared_between_views_and_invalidated_by_weather_sun_wind_and_large_moves() {
    let _gpu = test_support::serial();
    let Some(mut renderer) = test_support::headless((32, 32)) else { return };
    let source = texture(&mut renderer, "unused authored source", 1, &[255; 4]);
    let material = renderer
        .create_sky_material(&SkyMaterialDesc { kind: SkyLayerKind::Base, diffuse: source, miscellaneous: source })
        .unwrap();
    let mesh = mesh(&mut renderer, material, [0.5; 2]);
    let state = SkyState { enhanced: true, cloud_coverage: 0.5, ..Default::default() };
    let mut view = frame();
    let commands = [Instance::new(mesh, Mat4::IDENTITY)];
    renderer.set_sky_state(state);
    renderer.render(&view, &commands).unwrap();
    assert_eq!(renderer.sky.as_ref().unwrap().cloud_map.updates, 1);
    // Cubemap faces / mirror directions and nearby eyes must reuse the cloud march.
    view.view = Mat4::from_rotation_z(0.5);
    view.camera_position = Vec3::new(3.0, 2.0, 1.0);
    renderer.render(&view, &commands).unwrap();
    assert_eq!(renderer.sky.as_ref().unwrap().cloud_map.updates, 1);
    view.camera_position = Vec3::X * 100.0;
    renderer.render(&view, &commands).unwrap();
    assert_eq!(renderer.sky.as_ref().unwrap().cloud_map.updates, 2);
    view.light_dir = Vec3::new(-1.0, 0.0, -0.2).normalize();
    renderer.render(&view, &commands).unwrap();
    assert_eq!(renderer.sky.as_ref().unwrap().cloud_map.updates, 3);
    renderer.set_sky_state(SkyState { scroll: 0.002, ..state });
    renderer.render(&view, &commands).unwrap();
    assert_eq!(renderer.sky.as_ref().unwrap().cloud_map.updates, 4, "no fixed-rate wind updates");
    renderer.set_sky_state(SkyState { scroll: 0.002, cloud_coverage: 1.0, ..state });
    renderer.render(&view, &commands).unwrap();
    assert_eq!(renderer.sky.as_ref().unwrap().cloud_map.updates, 5);
    renderer.set_sky_state(SkyState::default());
    renderer.render(&view, &commands).unwrap();
    assert_eq!(renderer.sky.as_ref().unwrap().cloud_map.updates, 5, "authored sky never generates clouds");
    renderer.set_sky_state(SkyState { sky_wind_speed: 0.0, ..state });
    renderer.render(&view, &commands).unwrap();
    let stopped = renderer.read_output().unwrap().unwrap();
    assert_eq!(renderer.sky.as_ref().unwrap().cloud_map.updates, 6);
    renderer.set_sky_state(SkyState { scroll: 200.0, sky_wind_speed: 0.0, ..state });
    renderer.render(&view, &commands).unwrap();
    assert_eq!(renderer.sky.as_ref().unwrap().cloud_map.updates, 6, "zero speed holds the cloud field");
    assert_eq!(stopped, renderer.read_output().unwrap().unwrap());
    renderer.set_sky_state(SkyState { sky_wind_speed: 0.0, sky_brightness: 0.25, ..state });
    renderer.render(&view, &commands).unwrap();
    assert_eq!(renderer.sky.as_ref().unwrap().cloud_map.updates, 6, "exposure does not require another march");
    assert_ne!(stopped, renderer.read_output().unwrap().unwrap(), "brightness previews immediately");
}

#[test]
#[ignore = "needs a GPU"]
fn low_sun_warms_the_sunward_horizon_without_warming_the_noon_sky() {
    let _gpu = test_support::serial();
    let Some(mut renderer) = test_support::headless((32, 32)) else { return };
    let source = texture(&mut renderer, "unused authored source", 1, &[255; 4]);
    let material = renderer
        .create_sky_material(&SkyMaterialDesc { kind: SkyLayerKind::Base, diffuse: source, miscellaneous: source })
        .unwrap();
    let mesh = mesh(&mut renderer, material, [0.5; 2]);
    renderer.set_sky_state(SkyState { enhanced: true, ..Default::default() });
    let commands = [Instance::new(mesh, Mat4::IDENTITY)];
    let mut view = frame();
    view.camera_position = Vec3::Z * 0.45;
    renderer.render(&view, &commands).unwrap();
    let noon = renderer.read_output().unwrap().unwrap().pixel(31, 16).unwrap();
    view.light_dir = -Vec3::new(0.986, 0.0, 0.16).normalize();
    renderer.render(&view, &commands).unwrap();
    let low = renderer.read_output().unwrap().unwrap().pixel(31, 16).unwrap();
    assert!(noon[2] > noon[0], "noon should remain blue: {noon:?}");
    assert!(low[0] > low[2], "the low-sun horizon should be warm: {low:?}");
}

#[test]
#[ignore = "needs a GPU"]
fn solar_disk_retains_sdr_contrast_and_survives_hdr_sdr_switching() {
    use crate::gpu::post::display::Mapping;
    let _gpu = test_support::serial();
    let Some(mut renderer) = test_support::headless((64, 64)) else { return };
    let source = texture(&mut renderer, "unused authored source", 1, &[255; 4]);
    let material = renderer
        .create_sky_material(&SkyMaterialDesc { kind: SkyLayerKind::Base, diffuse: source, miscellaneous: source })
        .unwrap();
    let mesh = mesh(&mut renderer, material, [0.5; 2]);
    let sun = Vec3::new(0.98, 0.0, 0.2).normalize();
    let model =
        Mat4::from_quat(glam::Quat::from_rotation_arc(Vec3::Z, sun)) * Mat4::from_scale(Vec3::new(0.02, 0.02, 1.0));
    let mut view = frame();
    view.view = model.inverse();
    view.light_dir = -sun;
    // A deliberately over-range haze fixture recreates clipping of bright cloud/sky light.
    view.clear_color = [2.0; 3];
    let commands = [Instance::new(mesh, model)];
    renderer.set_sky_state(SkyState { enhanced: true, ..Default::default() });
    renderer.render(&view, &commands).unwrap();
    let sdr = renderer.read_output().unwrap().unwrap();
    let disk = sdr.pixel(32, 32).unwrap();
    let haze = sdr.pixel(32, 50).unwrap();
    assert!((0..3).all(|c| disk[c] > haze[c] + 8), "disk {disk:?} must stand out from bright haze {haze:?}");
    renderer.post.set_display(&renderer.device, Some(Mapping { peak: 8.0, white: 203.0, linear: true }));
    renderer.refresh_targets();
    let hdr = renderer.capture_hdr_linear(64, 64, &view, &commands).unwrap();
    assert!(hdr.iter().flatten().all(|v| v.is_finite()));
    assert!(hdr[32 * 64 + 32][0] > 1.0, "HDR must retain above-white radiance");
    renderer.post.set_display(&renderer.device, None);
    renderer.refresh_targets();
    renderer.render(&view, &commands).unwrap();
    let restored = renderer.read_output().unwrap().unwrap();
    assert_eq!(sdr, restored, "switching back must restore the exact SDR sky without stale state");
}
