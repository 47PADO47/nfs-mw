//! Synthetic contact and integration tests; no installed game assets.

use blackbox_gfx::{
    AoQuality, AoSettings, BlendMode, FrameParams, GraphicsSettings, Instance, Projection, RenderBackend, Shading,
    Upscaler,
};
use blackbox_gfx_testkit::mesh::{MeshBuilder, prelit};
use glam::{Mat4, Vec3};

use crate::Renderer;
use crate::gpu::test_support;

fn scene(renderer: &mut Renderer, size: (u32, u32), with_box: bool) -> (FrameParams, Vec<Instance>) {
    let mut mesh = MeshBuilder::new();
    mesh.state(None, BlendMode::Opaque, Shading::Prelit);
    let color = prelit([0.65; 3], 1.0);
    mesh.flat_quad(
        [
            Vec3::new(-10.0, -10.0, 0.0),
            Vec3::new(10.0, -10.0, 0.0),
            Vec3::new(10.0, 10.0, 0.0),
            Vec3::new(-10.0, 10.0, 0.0),
        ],
        color,
    );
    if with_box {
        mesh.cuboid(Vec3::new(-0.7, -0.7, 0.0), Vec3::new(0.7, 0.7, 1.4), color);
    }
    let handle = mesh.finish().upload(renderer, "AO contact scene");
    let eye = Vec3::new(3.0, -5.0, 3.0);
    let frame = FrameParams {
        view: glam::camera::rh::view::look_at_mat4(eye, Vec3::new(0.0, 0.0, 0.4), Vec3::Z),
        projection: Projection::PerspectiveInfiniteReverse {
            fov_y: 1.0,
            aspect: size.0 as f32 / size.1 as f32,
            near: 0.1,
        },
        camera_position: eye,
        light_dir: Vec3::NEG_Z,
        clear_color: [0.2, 0.3, 0.4],
        fog: None,
        camera_cut: false,
    };
    (frame, vec![Instance::new(handle, Mat4::IDENTITY)])
}

fn set(renderer: &mut Renderer, quality: AoQuality, compare: bool) {
    renderer.apply_graphics(&GraphicsSettings {
        ao: AoSettings { quality, compare, ..AoSettings::default() },
        ..GraphicsSettings::default()
    });
}

#[test]
#[ignore = "needs a GPU"]
fn ao_contacts_are_bounded_and_off_round_trips_exactly() {
    let _serial = test_support::serial();
    let size = (320, 240);
    let Some(mut renderer) = test_support::headless(size) else { return };
    let (frame, instances) = scene(&mut renderer, size, true);
    let off = renderer.capture(size.0, size.1, &frame, &instances).unwrap();
    assert!(renderer.ao.is_none());
    for quality in [AoQuality::Low, AoQuality::Medium, AoQuality::High] {
        set(&mut renderer, quality, false);
        let on = renderer.capture(size.0, size.1, &frame, &instances).unwrap();
        assert_eq!(
            on,
            renderer.capture(size.0, size.1, &frame, &instances).unwrap(),
            "fixed sampling is deterministic"
        );
        let mut moved = frame;
        moved.view = Mat4::from_translation(Vec3::new(0.4, -0.2, 0.0)) * frame.view;
        renderer.capture(size.0, size.1, &moved, &instances).unwrap();
        moved = frame;
        moved.camera_cut = true;
        assert_eq!(
            on,
            renderer.capture(size.0, size.1, &moved, &instances).unwrap(),
            "motion and cuts leave no history"
        );
        let mut darkened = 0;
        for (a, b) in off.as_chunks::<4>().0.iter().zip(on.as_chunks::<4>().0) {
            assert_eq!(a[3], b[3], "alpha is preserved");
            assert!(b[0] <= a[0], "AO cannot brighten");
            assert!(f32::from(b[0]) >= f32::from(a[0]) * 0.65 - 1.0, "bounded darkening");
            darkened += usize::from(a[0] > b[0] + 3);
        }
        assert!(darkened > 100, "{quality}: contact shading is visible ({darkened} pixels)");
        assert_eq!(&off[..320 * 4], &on[..320 * 4], "sky stays unchanged");
        set(&mut renderer, quality, true);
        let split = renderer.capture(size.0, size.1, &frame, &instances).unwrap();
        for y in 0..size.1 as usize {
            let at = y * size.0 as usize * 4;
            assert_eq!(&split[at..at + 160 * 4], &off[at..at + 160 * 4]);
            assert_eq!(&split[at + 160 * 4..at + 320 * 4], &on[at + 160 * 4..at + 320 * 4]);
        }
    }
    set(&mut renderer, AoQuality::Off, false);
    assert_eq!(off, renderer.capture(size.0, size.1, &frame, &instances).unwrap());
    assert!(renderer.ao.is_none(), "off releases the AO resources");
}

#[test]
#[ignore = "needs a GPU"]
fn ao_survives_render_scale_resize_and_projection_changes() {
    let _serial = test_support::serial();
    let Some(mut renderer) = test_support::headless((319, 237)) else { return };
    let (mut frame, instances) = scene(&mut renderer, (319, 237), true);
    for (size, quality, scale) in
        [((319, 237), AoQuality::Medium, 0.67), ((101, 79), AoQuality::High, 1.0), ((1, 1), AoQuality::Low, 1.0)]
    {
        renderer.apply_graphics(&GraphicsSettings {
            ao: AoSettings { quality, ..AoSettings::default() },
            upscaler: Upscaler::Bilinear,
            render_scale: scale,
            ..GraphicsSettings::default()
        });
        frame.projection =
            Projection::PerspectiveInfiniteReverse { fov_y: 1.0, aspect: size.0 as f32 / size.1 as f32, near: 0.1 };
        frame.camera_cut = true;
        assert_eq!(renderer.capture(size.0, size.1, &frame, &instances).unwrap().len(), (size.0 * size.1 * 4) as usize);
    }
    frame.projection = Projection::Identity;
    let on = renderer.capture(101, 79, &frame, &[]).unwrap();
    set(&mut renderer, AoQuality::Off, false);
    assert_eq!(on, renderer.capture(101, 79, &frame, &[]).unwrap(), "2D frames skip AO");
}

#[test]
#[ignore = "needs a GPU"]
fn ao_preserves_open_planes_ui_and_fully_fogged_surfaces() {
    use blackbox_gfx::{Fog, UiLayer, UiMesh, UiTextureId, UiTexturePatch, UiVertex};
    let _serial = test_support::serial();
    let size = (320, 240);
    let Some(mut renderer) = test_support::headless(size) else { return };
    let (frame, ground) = scene(&mut renderer, size, false);
    let off = renderer.capture(size.0, size.1, &frame, &ground).unwrap();
    for quality in [AoQuality::Low, AoQuality::Medium, AoQuality::High] {
        set(&mut renderer, quality, false);
        let on = renderer.capture(size.0, size.1, &frame, &ground).unwrap();
        assert_eq!(off, on, "{quality}: an open plane has no occluders, including at the screen edge");
    }
    let (mut frame, contacts) = scene(&mut renderer, size, true);
    let id = UiTextureId::from_raw(99);
    renderer.update_ui_texture(&UiTexturePatch { id, offset: None, size: [1, 1], rgba: &[255; 4] });
    renderer.set_ui_layer(UiLayer {
        pixels_per_point: 1.0,
        meshes: vec![UiMesh {
            vertices: [[100.0, 100.0], [200.0, 100.0], [200.0, 200.0], [100.0, 200.0]]
                .map(|position| UiVertex { position, uv: [0.0; 2], color_rgba: [220, 30, 60, 255] })
                .to_vec(),
            indices: vec![0, 1, 2, 0, 2, 3],
            texture: id,
            clip: [0.0, 0.0, 320.0, 240.0],
        }],
    });
    let on = renderer.capture(size.0, size.1, &frame, &contacts).unwrap();
    let pixel = (150 * 320 + 150) * 4;
    assert_eq!(&on[pixel..pixel + 4], &[220, 30, 60, 255], "UI is drawn after AO");
    frame.fog = Some(Fog { start: 0.0, end: 0.1 });
    let fogged = renderer.capture(size.0, size.1, &frame, &contacts).unwrap();
    set(&mut renderer, AoQuality::Off, false);
    assert_eq!(fogged, renderer.capture(size.0, size.1, &frame, &contacts).unwrap(), "AO fades with fog");
    renderer.apply_graphics(&GraphicsSettings {
        ao: AoSettings { quality: AoQuality::High, intensity: 0.0, ..Default::default() },
        ..Default::default()
    });
    renderer.capture(size.0, size.1, &frame, &contacts).unwrap();
    assert!(renderer.ao.is_none(), "zero strength does no AO work");
}
