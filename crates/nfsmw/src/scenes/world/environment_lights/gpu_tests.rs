//! Native GPU output for environment blink flares and scene depth occlusion.

use blackbox_gfx::{
    AoQuality, BlendMode, DrawRange, EffectLayer, FrameParams, GraphicsApi, Instance, MeshDesc, PixelFormat,
    ReflectionQuality, RenderBackend, Shading, ShadowQuality, TextureDesc, Vertex,
};
use blackbox_render::{Renderer, RendererOptions};
use glam::{Mat4, Vec3};

#[test]
#[ignore = "requires a Vulkan GPU"]
fn environment_flare_blink_and_scene_depth_vulkan() {
    run(GraphicsApi::Vulkan);
}

#[cfg(target_os = "windows")]
#[test]
#[ignore = "requires a DX12 GPU"]
fn environment_flare_blink_and_scene_depth_dx12() {
    run(GraphicsApi::Dx12);
}

fn run(api: GraphicsApi) {
    let mut renderer = Renderer::headless(
        (128, 128),
        RendererOptions { backend: api, vsync: false, force_fallback_adapter: false, pipeline_cache_dir: None },
    )
    .expect("selected GPU API must be available");
    let mut graphics = *renderer.graphics();
    graphics.post = Default::default();
    graphics.ao.quality = AoQuality::Off;
    graphics.shadows = ShadowQuality::Off;
    graphics.reflections = ReflectionQuality::Off;
    renderer.apply_graphics(&graphics);
    let texture = renderer.create_texture(&TextureDesc {
        label: "synthetic white environment flare",
        width: 1,
        height: 1,
        format: PixelFormat::Rgba8,
        mips: vec![&[255; 4]],
    });
    let (mut lights, mut textures, mut view) = super::tests::fixture();
    textures.get_mut(&1).unwrap().0 = texture;
    view.height = 128;
    let lens = crate::viewer::camera::Lens::look_at(view.camera, Vec3::ZERO, view.fov_degrees, 0.1, 1.0);
    let frame = FrameParams {
        view: lens.view,
        projection: lens.projection,
        camera_position: view.camera,
        light_dir: Vec3::NEG_Z,
        clear_color: [0.0; 3],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: true,
        shadow_tuning: None,
    };
    let vertices = [[5.0, -2.0, -2.0], [5.0, 2.0, -2.0], [5.0, 2.0, 2.0], [5.0, -2.0, 2.0]].map(|position| Vertex {
        position,
        normal: [1.0, 0.0, 0.0],
        color_bgra: [0, 0, 0, 255],
        uv: [0.0; 2],
    });
    let wall = renderer.create_mesh(&MeshDesc {
        label: "synthetic black scene depth occluder",
        vertices: &vertices,
        secondary_uvs: None,
        tangents: None,
        indices: &[0, 1, 2, 0, 2, 3, 2, 1, 0, 3, 2, 0],
        draws: vec![DrawRange {
            first_index: 0,
            index_count: 12,
            base_vertex: 0,
            texture: None,
            blend: BlendMode::Opaque,
            shading: Shading::Prelit,
            two_sided: false,
        }],
    });
    let capture = |renderer: &mut Renderer, lights: &super::EnvironmentLights, instances: &[Instance]| {
        let mut effects = EffectLayer::default();
        lights.draw(&textures, view, &mut effects.textured);
        renderer.set_effects(&effects);
        renderer.render(&frame, instances).unwrap();
        renderer.read_output().expect("headless output").unwrap()
    };
    let on = capture(&mut renderer, &lights, &[]);
    let blocked = capture(&mut renderer, &lights, &[Instance::new(wall, Mat4::IDENTITY)]);
    lights.blink.advance(0.6);
    let off = capture(&mut renderer, &lights, &[]);
    let bright = |image: &blackbox_gfx::RgbaImage| image.rgba.as_chunks::<4>().0.iter().filter(|p| p[0] > 20).count();
    assert!(bright(&on) > 20, "the environment flare must reach visible native pixels");
    assert_eq!(bright(&off), 0, "the off phase must remove native flare pixels");
    assert_eq!(bright(&blocked), 0, "opaque scene depth must occlude the flare");
    lights.blink.advance(0.6);
    let restored = capture(&mut renderer, &lights, &[]);
    assert_eq!(on.rgba, restored.rgba, "the next lit phase restores the flare");
    renderer.destroy_mesh(wall);
    renderer.destroy_texture(texture);
}
