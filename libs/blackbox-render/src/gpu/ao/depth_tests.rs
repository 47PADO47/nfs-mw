//! Depth reuse must preserve cutout coverage, transparent layering and biased mip selection.
use super::tests::scene_shading;
use crate::gpu::test_support;
use blackbox_gfx::{
    AoQuality, AoSettings, BlendMode, GlossyMaterial, GraphicsSettings, Instance, PixelFormat, RenderBackend, Shading,
    TextureDesc, Upscaler,
};
use blackbox_gfx_testkit::mesh::{MeshBuilder, prelit};
use glam::{Mat4, Vec3};

#[test]
#[ignore = "needs a GPU"]
fn depth_reuse_preserves_cutout_transparency_and_falls_back_for_biased_mips() {
    let _serial = test_support::serial();
    let size = (319, 237);
    let Some(mut r) = test_support::headless(size) else { return };
    let material = r.create_glossy_material(&GlossyMaterial::default());
    // Different alpha coverage at each mip makes a prepass/colour LOD mismatch observable.
    let mips: Vec<Vec<u8>> = (0..5)
        .map(|level| {
            let width = 16 >> level;
            (0..width * width)
                .flat_map(|pixel| {
                    let alpha = match (pixel / width + pixel % width + level) % 2 {
                        0 => 0,
                        _ => 255,
                    };
                    [230, 90, 30, alpha]
                })
                .collect()
        })
        .collect();
    let texture = r.create_texture(&TextureDesc {
        label: "depth reuse cutout mips",
        width: 16,
        height: 16,
        format: PixelFormat::Rgba8,
        mips: mips.iter().map(Vec::as_slice).collect(),
    });
    for shading in [Shading::Lit, Shading::Prelit, Shading::Glossy(material)] {
        let (frame, mut instances) = scene_shading(&mut r, size, false, Shading::Prelit);
        let mut mesh = MeshBuilder::new();
        mesh.state(Some(texture), BlendMode::AlphaTest, shading);
        mesh.cuboid(Vec3::new(-0.8, -0.8, 0.0), Vec3::new(0.8, 0.8, 1.8), prelit([0.7; 3], 1.0));
        // Invisible vertex opacity in scenery differs from the glossy texture-only cutoff.
        mesh.cuboid(Vec3::new(-2.0, -0.8, 0.0), Vec3::new(-1.0, 0.8, 1.8), prelit([0.7; 3], 0.0));
        mesh.state(None, BlendMode::AlphaBlend, Shading::Prelit);
        mesh.card(Vec3::new(0.0, -1.5, 0.0), 4.0, 2.0, 0.0, prelit([0.2, 0.8, 0.3], 0.4));
        let handle = mesh.finish().upload(&mut r, "layered cutout scene");
        instances.push(Instance::new(handle, Mat4::from_rotation_z(0.13)));
        for scale in [1.0, 0.67] {
            r.apply_graphics(&GraphicsSettings {
                ao: AoSettings { quality: AoQuality::High, ..Default::default() },
                upscaler: Upscaler::Bilinear,
                render_scale: scale,
                ..Default::default()
            });
            r.set_prepass_depth_reuse(true);
            r.render(&frame, &instances).unwrap();
            let shared = r.read_output().unwrap().unwrap().rgba;
            let depth_is_shared = r.surfaces.as_ref().unwrap().targets.depth == *r.targets.depth_view();
            assert_eq!(depth_is_shared, scale == 1.0, "biased alpha-test mips require separate depth");
            r.set_prepass_depth_reuse(false);
            r.render(&frame, &instances).unwrap();
            assert_eq!(shared, r.read_output().unwrap().unwrap().rgba, "changed {shading:?} at scale {scale}");
        }
    }
}
