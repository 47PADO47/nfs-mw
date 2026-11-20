//! Actual raster checks of the panoramic sky: the domes replace the procedural clouds, blend, and the moon is a picture.
use super::*;
use blackbox_gfx::{SkyPanorama, SkyPanoramaState};

fn state() -> SkyPanoramaState {
    SkyPanoramaState {
        blend: 0.0,
        rotation: [0.0; 2],
        tint: [[1.0; 3]; 2],
        gain: 1.0,
        moon_size: 0.0173,
        moon_brightness: 1.0,
    }
}

fn image(renderer: &mut Renderer, mesh: blackbox_gfx::MeshHandle, state: SkyState) -> Vec<[u8; 4]> {
    renderer.set_sky_state(state);
    renderer.render(&frame(), &[Instance::new(mesh, Mat4::IDENTITY)]).unwrap();
    let output = renderer.read_output().unwrap().unwrap();
    (0..32).flat_map(|y| (0..32).map(move |x| (x, y))).map(|(x, y)| output.pixel(x, y).unwrap()).collect()
}

fn sky(renderer: &mut Renderer) -> blackbox_gfx::MeshHandle {
    let source = texture(renderer, "unused authored source", 1, &[255; 4]);
    let material = renderer
        .create_sky_material(&SkyMaterialDesc { kind: SkyLayerKind::Base, diffuse: source, miscellaneous: source })
        .unwrap();
    mesh(renderer, material, [0.5; 2])
}

#[test]
#[ignore = "needs a GPU"]
fn a_dome_replaces_the_procedural_clouds_a_clear_dome_leaves_the_clear_sky_and_the_blend_moves_between() {
    let _gpu = test_support::serial();
    let Some(mut renderer) = test_support::headless((32, 32)) else { return };
    let mesh = sky(&mut renderer);
    let red = texture(&mut renderer, "red dome", 1, &[255, 0, 0, 255]);
    let clear = texture(&mut renderer, "clear dome", 1, &[0, 0, 0, 0]);
    let base = SkyState { enhanced: true, ..Default::default() };
    let procedural = image(&mut renderer, mesh, base);
    renderer.set_sky_panorama(Some(SkyPanorama { a: red, b: clear, moon: None }));
    let with = |blend: f32| SkyState { panorama: Some(SkyPanoramaState { blend, ..state() }), ..base };
    let first = image(&mut renderer, mesh, with(0.0));
    assert!(first.iter().all(|p| p[0] > 200 && p[1] < 40), "an opaque red dome covers the sky: {:?}", first[500]);
    let second = image(&mut renderer, mesh, with(1.0));
    for (a, b) in second.iter().zip(&procedural) {
        for channel in 0..3 {
            assert!(a[channel].abs_diff(b[channel]) <= 3, "a transparent dome shows the sky: {a:?} {b:?}");
        }
    }
    let half = image(&mut renderer, mesh, with(0.5));
    let mean = |image: &[[u8; 4]], channel: usize| {
        image.iter().map(|p| f32::from(p[channel])).sum::<f32>() / image.len() as f32
    };
    assert!(
        mean(&half, 1) > mean(&first, 1) + 20.0,
        "half way the sky shows through: {:?} of {:?}",
        half[500],
        first[500]
    );
    assert!(mean(&half, 0) > 50.0 && mean(&half, 0) < mean(&first, 0), "and the red is less than a whole dome's");
    let tinted = image(
        &mut renderer,
        mesh,
        SkyState { panorama: Some(SkyPanoramaState { tint: [[0.1; 3]; 2], ..state() }), ..base },
    );
    assert!(tinted.iter().zip(&first).all(|(t, f)| t[0] < f[0]), "a darker tint darkens the clouds");
    renderer.set_sky_panorama(None);
    let back = image(&mut renderer, mesh, SkyState { panorama: Some(state()), ..base });
    let plain = image(&mut renderer, mesh, base);
    assert_eq!(back, plain, "without pictures the panorama state is ignored");
}

#[test]
#[ignore = "needs a GPU"]
fn the_cloud_map_is_rebuilt_when_the_pictures_or_the_blend_change() {
    let _gpu = test_support::serial();
    let Some(mut renderer) = test_support::headless((32, 32)) else { return };
    let mesh = sky(&mut renderer);
    let a = texture(&mut renderer, "a", 1, &[255, 255, 255, 255]);
    let b = texture(&mut renderer, "b", 1, &[0, 0, 0, 0]);
    let base = SkyState { enhanced: true, ..Default::default() };
    renderer.set_sky_panorama(Some(SkyPanorama { a, b, moon: None }));
    let updates = |renderer: &Renderer| renderer.sky.as_ref().unwrap().cloud_map.updates;
    image(&mut renderer, mesh, SkyState { panorama: Some(state()), ..base });
    let first = updates(&renderer);
    image(&mut renderer, mesh, SkyState { panorama: Some(state()), ..base });
    assert_eq!(updates(&renderer), first, "the same state draws from the same map");
    image(&mut renderer, mesh, SkyState { panorama: Some(SkyPanoramaState { blend: 0.6, ..state() }), ..base });
    assert_eq!(updates(&renderer), first + 1, "a jump of the blend rebuilds every row");
    renderer.set_sky_panorama(Some(SkyPanorama { a: b, b: a, moon: None }));
    image(&mut renderer, mesh, SkyState { panorama: Some(SkyPanoramaState { blend: 0.6, ..state() }), ..base });
    assert_eq!(updates(&renderer), first + 2, "other pictures rebuild it");
}

#[test]
#[ignore = "needs a GPU"]
fn the_moon_is_the_picture_at_night_and_the_procedural_disc_without_one() {
    let _gpu = test_support::serial();
    let Some(mut renderer) = test_support::headless((32, 32)) else { return };
    let mesh = sky(&mut renderer);
    let clear = texture(&mut renderer, "clear dome", 1, &[0, 0, 0, 0]);
    let white = texture(&mut renderer, "moon", 1, &[255, 255, 255, 255]);
    let night = SkyState {
        enhanced: true,
        night_amount: 1.0,
        moon_direction: [0.0, 0.0, 1.0],
        celestial_sun: Some([0.0, 0.0, -1.0]),
        panorama: Some(SkyPanoramaState { moon_size: 0.1, ..state() }),
        ..Default::default()
    };
    renderer.set_sky_panorama(Some(SkyPanorama { a: clear, b: clear, moon: None }));
    let without = image(&mut renderer, mesh, night);
    renderer.set_sky_panorama(Some(SkyPanorama { a: clear, b: clear, moon: Some(white) }));
    let with = image(&mut renderer, mesh, night);
    let centre = |image: &[[u8; 4]]| image[16 * 32 + 16];
    assert!(centre(&with)[0] > centre(&without)[0] + 50, "{:?} {:?}", centre(&with), centre(&without));
}
