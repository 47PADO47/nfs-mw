//! Spatial water response: original pixels, vertical receivers, stable patches and rain animation.
use super::*;

fn draw(r: &mut Renderer, mesh: MeshHandle, water: SurfaceWater, vertical: bool) -> RgbaImage {
    r.set_surface_water(water);
    let mut f = frame();
    f.view = Mat4::from_scale(Vec3::new(0.05, 0.05, 1.0));
    f.camera_position = Vec3::new(0.0, -50.0, 8.0);
    let light = f.surface_light.as_mut().unwrap();
    light.ambient = [0.3; 3];
    light.diffuse = [0.4; 3];
    light.specular = [0.8; 3];
    let transform = match vertical {
        true => Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2),
        false => Mat4::IDENTITY,
    };
    f.view *= transform.inverse();
    r.render(&f, &[Instance::new(mesh, transform)]).unwrap();
    r.read_output().unwrap().unwrap()
}

#[test]
#[ignore = "requires a GPU"]
fn enhanced_water_keeps_dry_pixels_and_produces_stable_patches() {
    let _gpu = test_support::serial();
    let mut r = Renderer::headless((128, 128), test_support::options()).unwrap();
    let mut options = RenderExperiments { original_world_materials: true, ..Default::default() };
    r.set_render_experiments(options);
    let asphalt = upload(&mut r, [110, 100, 90, 255]);
    let flat = upload(&mut r, [128, 128, 255, 255]);
    let mask = upload(&mut r, [255; 4]);
    let sky = upload(&mut r, [120, 155, 190, 255]);
    let material = material(&mut r, [asphalt, flat, mask], Some(sky));
    let mesh = quad(&mut r, [-20.0, -20.0, 20.0, 20.0], [255; 4], asphalt, Shading::WorldNormal(material));
    let dry = draw(&mut r, mesh, SurfaceWater::default(), false);
    options.enhanced_wet_surfaces = true;
    r.set_render_experiments(options);
    assert_eq!(dry.rgba, draw(&mut r, mesh, SurfaceWater::default(), false).rgba);
    let water = SurfaceWater { dampness: 1.0, pooling: 1.0, time: 10.0, rain: 1.0 };
    let pooled = draw(&mut r, mesh, water, false);
    assert_eq!(pooled.rgba, draw(&mut r, mesh, water, false).rgba, "no temporal history or camera dither");
    let film = draw(&mut r, mesh, SurfaceWater { pooling: 0.0, ..water }, false);
    let changed = film
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .zip(pooled.rgba.as_chunks::<4>().0)
        .filter(|(a, b)| a[..3] != b[..3])
        .count();
    assert!(changed > 100, "pooled water must differ from the damp film: {changed}");
    assert_ne!(
        pooled.rgba,
        draw(&mut r, mesh, SurfaceWater { time: 11.0, ..water }, false).rgba,
        "rain ripples animate"
    );
    let stopped = SurfaceWater { rain: 0.0, ..water };
    assert_eq!(
        draw(&mut r, mesh, stopped, false).rgba,
        draw(&mut r, mesh, SurfaceWater { time: 11.0, ..stopped }, false).rgba,
        "rain ripples stop after rainfall"
    );
    let wall = draw(&mut r, mesh, water, true);
    options.enhanced_wet_surfaces = false;
    r.set_render_experiments(options);
    assert_eq!(wall.rgba, draw(&mut r, mesh, water, true).rgba, "walls retain authored shading");
}

#[test]
#[ignore = "requires a GPU"]
fn hybrid_road_reflects_visible_geometry_and_preserves_foreground() {
    use blackbox_gfx_testkit::mesh::{MeshBuilder, prelit};
    let _gpu = test_support::serial();
    let mut r = Renderer::headless((192, 144), test_support::options()).unwrap();
    let black = upload(&mut r, [0, 0, 0, 255]);
    let flat = upload(&mut r, [128, 128, 255, 255]);
    let mask = upload(&mut r, [255; 4]);
    let road = material(&mut r, [black, flat, mask], None);
    let road = quad(&mut r, [-4.0, -4.0, 4.0, 4.0], [255; 4], black, Shading::WorldNormal(road));
    let road = Instance::new(road, Mat4::from_translation(Vec3::new(0.0, 0.0, -0.5)));
    let mut wall = MeshBuilder::new();
    wall.state(None, BlendMode::Opaque, Shading::Prelit);
    wall.flat_quad(
        [Vec3::new(-3.0, 3.0, 0.0), Vec3::new(-3.0, 3.0, 5.0), Vec3::new(3.0, 3.0, 5.0), Vec3::new(3.0, 3.0, 0.0)],
        prelit([0.9, 0.0, 0.0], 1.0),
    );
    let wall = Instance::new(wall.finish().upload(&mut r, "road red reflector"), Mat4::IDENTITY);
    // Glossy draws occupy the matrix prefix; scenery must read the slot region after it.
    // A far-away first transform prevents an incorrect slot-zero read from passing by coincidence.
    let glossy = r.create_glossy_material(&GlossyMaterial::default());
    let marker = quad(&mut r, [-1.0, -1.0, 1.0, 1.0], [255; 4], black, Shading::Glossy(glossy));
    let marker = Instance::new(marker, Mat4::from_translation(Vec3::new(40.0, 0.0, 0.0)));

    let black_cube = [0, 0, 0, 255];
    r.set_environment(Environment::Faces { size: 1, faces: [&black_cube; 6] });
    let eye = Vec3::new(0.0, -5.0, 3.0);
    let mut f = frame();
    f.view = glam::camera::rh::view::look_at_mat4(eye, Vec3::new(0.0, 1.0, 0.5), Vec3::Z);
    f.camera_position = eye;
    f.projection = Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect: 192.0 / 144.0, near: 0.1 };
    let options =
        RenderExperiments { original_world_materials: true, enhanced_wet_surfaces: true, ..Default::default() };
    r.set_render_experiments(options);
    r.set_surface_water(SurfaceWater { dampness: 1.0, pooling: 1.0, ..Default::default() });
    r.render(&f, &[marker, road]).unwrap();
    let miss = r.read_output().unwrap().unwrap();
    r.render(&f, &[marker, road, wall]).unwrap();
    let hit = r.read_output().unwrap().unwrap();
    // Below the wall's projected base: only its reflection can add red here.
    let count = (90..140)
        .flat_map(|y| (40..152).map(move |x| (x, y)))
        .filter(|&(x, y)| {
            let a = miss.pixel(x, y).unwrap();
            let b = hit.pixel(x, y).unwrap();
            b[0] > a[0].saturating_add(20) && b[0] > b[1].saturating_add(20)
        })
        .count();
    assert!(count > 80, "visible wall must reflect on road: {count}");
    assert_eq!(
        hit.rgba,
        {
            r.render(&f, &[marker, road, wall]).unwrap();
            r.read_output().unwrap().unwrap().rgba
        },
        "no frame history"
    );
    r.set_render_experiments(RenderExperiments { enhanced_wet_surfaces: false, ..options });
    r.render(&f, &[marker, road, wall]).unwrap();
    let original = r.read_output().unwrap().unwrap();
    for y in 20..65 {
        for x in 70..120 {
            assert_eq!(hit.pixel(x, y), original.pixel(x, y), "foreground geometry must survive road redraw");
        }
    }
    r.set_render_experiments(options);
    r.render(&f, &[marker, road]).unwrap();
    assert_eq!(miss.rgba, r.read_output().unwrap().unwrap().rgba, "removed objects leave no reflection history");
    let scale = Mat4::from_scale(Vec3::splat(20.0));
    let scene = [marker, road, wall].map(|mut instance| {
        instance.transform = scale * instance.transform;
        instance
    });
    f.camera_position = eye * 20.0;
    f.view = glam::camera::rh::view::look_at_mat4(f.camera_position, Vec3::new(0.0, 20.0, 10.0), Vec3::Z);
    let reflected_red = |image: &RgbaImage| -> u64 {
        (90..140)
            .flat_map(|y| (40..152).map(move |x| (x, y)))
            .map(|(x, y)| {
                let p = image.pixel(x, y).unwrap();
                u64::from(p[0].saturating_sub(p[1]))
            })
            .sum()
    };
    r.set_render_experiments(RenderExperiments { road_reflection_range: 80.0, ..options });
    r.render(&f, &scene).unwrap();
    let short = reflected_red(&r.read_output().unwrap().unwrap());
    r.set_render_experiments(RenderExperiments { road_reflection_range: 320.0, road_reflection_fade: 0.1, ..options });
    r.render(&f, &scene).unwrap();
    let long = reflected_red(&r.read_output().unwrap().unwrap());
    r.set_render_experiments(RenderExperiments { road_reflection_range: 320.0, road_reflection_fade: 0.9, ..options });
    r.render(&f, &scene).unwrap();
    let faded = reflected_red(&r.read_output().unwrap().unwrap());
    assert!(long > short + 500, "longer rays retain distant reflections: {short} -> {long}");
    assert!(faded > short && faded < long, "wide blend softens reflections: {short} < {faded} < {long}");
}
