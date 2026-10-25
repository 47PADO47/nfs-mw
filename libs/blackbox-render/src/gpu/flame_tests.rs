//! End-to-end volume integration and clipping, using a synthetic constant lookup.
use blackbox_gfx::{
    BlendMode, EffectLayer, FlameVolume, FrameParams, Instance, PixelFormat, Projection, Shading, TextureDesc,
};
use blackbox_gfx_testkit::mesh::{MeshBuilder, prelit};
use glam::{Mat4, Quat, Vec3};

#[test]
#[ignore = "needs a GPU"]
fn flame_volumes_clip_at_opaque_surfaces_and_change_with_time() {
    let _gpu = super::test_support::serial();
    let mut renderer = super::test_support::headless((128, 128)).expect("GPU adapter");
    let pixels = [160, 80, 20, 255].repeat(4);
    let texture = renderer.create_texture(&TextureDesc {
        label: "synthetic flame lookup",
        width: 2,
        height: 2,
        format: PixelFormat::Rgba8,
        mips: vec![&pixels],
    });
    let volume = FlameVolume {
        texture,
        to_world: Mat4::from_scale_rotation_translation(
            Vec3::new(1.0, 2.0, 1.0),
            Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2),
            Vec3::new(4.0, 0.0, 0.0),
        ),
        time: 0.25,
        seed: 0.0,
        blue: true,
        intensity: 0.1,
        appearance: Default::default(),
    };
    let mut layer = EffectLayer::default();
    layer.flames.push(volume);
    let frame = FrameParams {
        view: glam::camera::rh::view::look_at_mat4(Vec3::ZERO, Vec3::X, Vec3::Z),
        projection: Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect: 1.0, near: 0.1 },
        camera_position: Vec3::ZERO,
        light_dir: Vec3::X,
        clear_color: [0.0; 3],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: false,
        shadow_tuning: None,
    };
    let mut wall = MeshBuilder::new();
    wall.state(None, BlendMode::Opaque, Shading::Prelit);
    wall.cuboid(Vec3::new(-0.01, -2.0, -2.0), Vec3::new(0.01, 2.0, 2.0), prelit([0.0; 3], 1.0));
    let wall = wall.finish().upload(&mut renderer, "opaque flame blocker");
    let mut shot = |frame: &FrameParams, layer: &EffectLayer, x: Option<f32>| {
        let instances: Vec<_> =
            x.map(|x| Instance::new(wall, Mat4::from_translation(Vec3::X * x))).into_iter().collect();
        renderer.set_effects(layer);
        renderer.render(frame, &instances).unwrap();
        renderer.read_output().unwrap().unwrap()
    };
    let energy =
        |image: &blackbox_gfx::RgbaImage| image.rgba.as_chunks::<4>().0.iter().map(|p| u64::from(p[2])).sum::<u64>();
    let full = shot(&frame, &layer, None);
    let partial = shot(&frame, &layer, Some(3.3));
    let hidden = shot(&frame, &layer, Some(2.5));
    assert!(energy(&full) > 1000, "the axial view must draw the volume");
    assert!(
        energy(&partial) > 0 && energy(&partial) < energy(&full),
        "a surface inside the volume clips its far samples"
    );
    assert_eq!(energy(&hidden), 0, "an opaque foreground surface hides the whole volume");
    layer.flames[0].time += 0.5;
    let later = shot(&frame, &layer, None);
    assert_ne!(full.rgba, later.rgba, "advected turbulence must animate");
    let mut side = frame;
    side.camera_position = Vec3::new(4.0, -3.0, 0.0);
    side.view = glam::camera::rh::view::look_at_mat4(side.camera_position, Vec3::new(4.0, 0.0, 0.0), Vec3::Z);
    assert!(energy(&shot(&side, &layer, None)) > 1000, "the side view must retain volume");
    let regular = energy(&shot(&frame, &layer, None));
    layer.flames[0].to_world *= Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0));
    let reflected = energy(&shot(&frame, &layer, None));
    assert!(regular.abs_diff(reflected) < regular / 20, "reflected source markers must retain the same flame");
    let mut inside = frame;
    inside.camera_position = layer.flames[0].to_world.transform_point3(Vec3::new(0.0, -0.45, 0.0));
    inside.view =
        glam::camera::rh::view::look_at_mat4(inside.camera_position, inside.camera_position + Vec3::X, Vec3::Z);
    assert!(energy(&shot(&inside, &layer, None)) > 0, "a camera inside a reflected volume must still see it");
    layer.flames[0].appearance.root_color = [0.0, 1.0, 0.0, 1.0];
    layer.flames[0].appearance.tail_color = [0.0, 1.0, 0.0, 1.0];
    layer.flames[0].appearance.bend = Vec3::X * 0.08;
    layer.flames[0].appearance.density = 1.4;
    let custom = shot(&frame, &layer, None);
    let channels = |channel| custom.rgba.as_chunks::<4>().0.iter().map(|p| u64::from(p[channel])).sum::<u64>();
    assert!(channels(1) > 1000, "custom green tint must reach the GPU");
    assert_eq!(channels(0) + channels(2), 0, "tint changes must not retain the original palette");
    assert_ne!(full.rgba, custom.rgba);
    layer.flames[0].appearance.root_color = [0.0, 0.0, 1.0, 1.0];
    layer.flames[0].appearance.tail_color = [0.0, 0.0, 1.0, 1.0];
    let blue = shot(&frame, &layer, None);
    let blue_energy = energy(&blue);
    assert!(
        blue_energy > channels(1) * 9 / 10 && blue_energy < channels(1) * 11 / 10,
        "equally bright custom hues must not inherit missing channels from the warm source texture"
    );
    layer.flames.clear();
    assert_eq!(energy(&shot(&frame, &layer, None)), 0, "clearing must remove uploaded volumes");
}
