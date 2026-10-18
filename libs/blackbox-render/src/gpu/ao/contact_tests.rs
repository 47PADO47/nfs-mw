//! Smooth contact shading around a raised body, without game textures or baked shadows.

use blackbox_gfx::{
    AoMethod, AoQuality, AoSettings, BlendMode, FrameParams, GraphicsSettings, Instance, Projection, RenderBackend,
    Shading,
};
use blackbox_gfx_testkit::mesh::{MeshBuilder, prelit};
use glam::{Mat4, Vec3};

use crate::{Renderer, gpu::test_support};

const SIZE: (u32, u32) = (1024, 768);

fn scene(renderer: &mut Renderer) -> (FrameParams, Vec<Instance>) {
    let mut mesh = MeshBuilder::new();
    mesh.state(None, BlendMode::Opaque, Shading::Prelit);
    mesh.flat_quad(
        [
            Vec3::new(-10.0, -10.0, 0.0),
            Vec3::new(10.0, -10.0, 0.0),
            Vec3::new(10.0, 10.0, 0.0),
            Vec3::new(-10.0, 10.0, 0.0),
        ],
        prelit([0.8; 3], 1.0),
    );
    mesh.cuboid(Vec3::new(-1.0, -2.2, 0.45), Vec3::new(1.0, 2.2, 1.2), prelit([0.5, 0.3, 0.2], 1.0));
    let handle = mesh.finish().upload(renderer, "AO raised body contact");
    let eye = Vec3::new(0.0, -8.0, 3.5);
    let frame = FrameParams {
        view: glam::camera::rh::view::look_at_mat4(eye, Vec3::new(0.0, 0.0, 0.5), Vec3::Z),
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
    (frame, vec![Instance::new(handle, Mat4::IDENTITY)])
}

#[test]
#[ignore = "needs a GPU"]
fn raised_body_has_smooth_ground_contact() {
    let _serial = test_support::serial();
    let Some(mut renderer) = test_support::headless(SIZE) else { return };
    let (frame, instances) = scene(&mut renderer);
    let baseline = renderer.capture(SIZE.0, SIZE.1, &frame, &instances).unwrap();
    let choices = [AoMethod::Gtao, AoMethod::XeGtao]
        .into_iter()
        .flat_map(|method| [AoQuality::High, AoQuality::Medium, AoQuality::Low].map(|quality| (method, quality)));
    for (method, quality, depth_mips) in
        choices.flat_map(|(method, quality)| [false, true].map(|depth_mips| (method, quality, depth_mips)))
    {
        renderer.apply_graphics(&GraphicsSettings {
            ao: AoSettings { method, quality, depth_mips, intensity: 1.0, ..Default::default() },
            ..Default::default()
        });
        let rgba = renderer.capture(SIZE.0, SIZE.1, &frame, &instances).unwrap();
        // The receiver is flat and uniformly lit. Sparse, coherent horizons used to
        // make separate hard bands here even at High, jumping 4-5 levels per row.
        let profile: Vec<f32> = (477..535)
            .map(|y| (448..576).map(|x| f32::from(rgba[(y * SIZE.0 as usize + x) * 4])).sum::<f32>() / 128.0)
            .collect();
        let gradient = profile.windows(2).map(|p| (p[1] - p[0]).abs()).fold(0.0, f32::max);
        assert!(gradient < 3.5, "{method}/{quality}: contact gradient has a hard band ({gradient})");
        let contrast = (477..535)
            .map(|y| {
                let before =
                    (448..576).map(|x| f32::from(baseline[(y * SIZE.0 as usize + x) * 4])).sum::<f32>() / 128.0;
                before - profile[y - 477]
            })
            .fold(0.0, f32::max);
        assert!(contrast > 1.0, "{quality}: ambient contact shading remains visible ({contrast})");
        // Compare the same ground points as the camera moves by subpixel amounts.
        // A repeated screen pattern must not replace the bands with visible shimmer.
        for shift in [0.005, 0.01, 0.02, 0.04] {
            let moved = FrameParams { view: Mat4::from_translation(Vec3::new(shift, 0.0, 0.0)) * frame.view, ..frame };
            let shifted = renderer.capture(SIZE.0, SIZE.1, &moved, &instances).unwrap();
            let mut errors = Vec::new();
            for y in 0..20 {
                for x in 0..20 {
                    let point = Vec3::new(-0.4 + x as f32 * 0.04, -1.65 - y as f32 * 0.04, 0.0);
                    errors.push((ground_sample(&rgba, &frame, point) - ground_sample(&shifted, &moved, point)).abs());
                }
            }
            errors.sort_by(f32::total_cmp);
            eprintln!(
                "{method}/{quality} depth_mips={depth_mips}: shift {shift} ground error mean {:.2}, p99 {:.2}",
                errors.iter().sum::<f32>() / errors.len() as f32,
                errors[396]
            );
            assert!(errors[396] < 4.0, "{method}/{quality}: contact shading shimmers during a small camera move");
        }
        if let Ok(directory) = std::env::var("BLACKBOX_AO_CAPTURE_DIR") {
            std::fs::write(
                std::path::Path::new(&directory).join(format!("ao-raised-{method}-{quality}-{depth_mips}.rgba")),
                &rgba,
            )
            .unwrap();
        }
    }
}

fn ground_sample(rgba: &[u8], frame: &FrameParams, point: Vec3) -> f32 {
    let clip = frame.projection.matrix() * frame.view * point.extend(1.0);
    let ndc = clip.truncate() / clip.w;
    let position = glam::Vec2::new((ndc.x + 1.0) * SIZE.0 as f32, (1.0 - ndc.y) * SIZE.1 as f32) * 0.5 - 0.5;
    let x = position.x.floor() as usize;
    let y = position.y.floor() as usize;
    let (fx, fy) = (position.x.fract(), position.y.fract());
    let sample = |x, y| f32::from(rgba[(y * SIZE.0 as usize + x) * 4]);
    let top = sample(x, y) * (1.0 - fx) + sample(x + 1, y) * fx;
    let bottom = sample(x, y + 1) * (1.0 - fx) + sample(x + 1, y + 1) * fx;
    top * (1.0 - fy) + bottom * fy
}
