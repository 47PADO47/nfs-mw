//! An elongated texture footprint must react live across retained material families.
use crate::gpu::{Renderer, test_support};
use blackbox_gfx::*;
use glam::{Mat4, Vec3};

fn striped_mips() -> Vec<Vec<u8>> {
    let mut levels = vec![
        (0..128 * 128)
            .flat_map(|i| match (i / 128 / 8) % 2 {
                0 => [40, 80, 120, 255],
                _ => [120, 80, 40, 255],
            })
            .collect::<Vec<_>>(),
    ];
    let mut edge = 128;
    while edge > 1 {
        let old = levels.last().unwrap();
        let half = edge / 2;
        let mut next = vec![0; half * half * 4];
        for y in 0..half {
            for x in 0..half {
                for c in 0..4 {
                    let sum: u32 = [(2 * x, 2 * y), (2 * x + 1, 2 * y), (2 * x, 2 * y + 1), (2 * x + 1, 2 * y + 1)]
                        .into_iter()
                        .map(|(u, v)| u32::from(old[(v * edge + u) * 4 + c]))
                        .sum();
                    next[(y * half + x) * 4 + c] = (sum / 4) as u8;
                }
            }
        }
        levels.push(next);
        edge = half;
    }
    levels
}
fn upload(r: &mut Renderer, label: &str, rgba: &[u8; 4]) -> TextureHandle {
    r.create_texture(&TextureDesc { label, width: 1, height: 1, format: PixelFormat::Rgba8, mips: vec![rgba] })
}
fn frame() -> FrameParams {
    FrameParams {
        view: Mat4::IDENTITY,
        projection: Projection::Identity,
        camera_position: Vec3::new(0.0, 0.0, 3.0),
        light_dir: Vec3::NEG_X,
        clear_color: [0.0; 3],
        fog: None,
        scattering: None,
        camera_cut: false,

        shadow_tuning: None,
        surface_light: Some(SurfaceLight {
            ambient: [0.3; 3],
            ambient_alpha: 1.0,
            diffuse: [0.0; 3],
            specular: [0.0; 3],
            specular_power: 11.0,
            reflection_scale: 0.0,
            reflective_specular_scale: 1.0,
            shadow_strength: 1.0,
        }),
    }
}
fn mesh(r: &mut Renderer, texture: TextureHandle, shading: Shading) -> MeshHandle {
    let vertices = [
        ([-1.0, -1.0, 0.5], [0.0, 0.0]),
        ([-1.0, 1.0, 0.5], [0.0, 0.53]),
        ([1.0, 1.0, 0.5], [16.0, 0.53]),
        ([1.0, -1.0, 0.5], [16.0, 0.0]),
    ]
    .map(|(position, uv)| Vertex { position, normal: [0.0, 0.0, 1.0], color_bgra: [128, 128, 128, 255], uv });
    r.create_mesh(&MeshDesc {
        label: "filtering footprint",
        vertices: &vertices,
        tangents: Some(&[[1.0, 0.0, 0.0, 1.0]; 4]),
        secondary_uvs: None,
        indices: &[0, 1, 2, 0, 2, 3],
        draws: vec![DrawRange {
            first_index: 0,
            index_count: 6,
            base_vertex: 0,
            texture: Some(texture),
            blend: BlendMode::Opaque,
            shading,
            two_sided: true,
        }],
    })
}

#[test]
#[ignore = "needs a GPU"]
fn scene_filtering_changes_live_and_round_trips_retained_materials() {
    let _guard = test_support::serial();
    let mut r = Renderer::headless((64, 64), test_support::options()).expect("selected GPU required");
    assert_eq!(r.capabilities().max_texture_anisotropy, 16, "fixture requires anisotropic filtering");
    r.set_render_experiments(RenderExperiments { original_world_materials: true, ..Default::default() });
    let original_counts = r.resource_counts();
    let levels = striped_mips();
    let texture = r.create_texture(&TextureDesc {
        label: "synthetic stripe mip chain",
        width: 128,
        height: 128,
        format: PixelFormat::Rgba8,
        mips: levels.iter().map(Vec::as_slice).collect(),
    });
    let flat = upload(&mut r, "flat tangent normal", &[128, 128, 255, 255]);
    let black = upload(&mut r, "black specular/reflection", &[0, 0, 0, 255]);
    let normal = r
        .create_normal_material(&NormalMaterialDesc {
            model: NormalModel::Bump,
            diffuse: texture,
            normal: flat,
            specular: black,
            reflection: None,
            uv_offset: [0.0; 2],
        })
        .unwrap();
    let reflected = r
        .create_normal_material(&NormalMaterialDesc {
            model: NormalModel::Reflective,
            diffuse: texture,
            normal: flat,
            specular: black,
            reflection: None,
            uv_offset: [0.0; 2],
        })
        .unwrap();
    let window = r.create_window_material(&WindowMaterialDesc { diffuse: texture, reflection: black }).unwrap();
    let sky = r
        .create_sky_material(&SkyMaterialDesc { diffuse: texture, miscellaneous: texture, kind: SkyLayerKind::Cloud2 })
        .unwrap();
    let glossy = r.create_glossy_material(&GlossyMaterial {
        diffuse_min: [0.5; 4],
        diffuse_range: [0.0; 4],
        specular_min: [0.0; 3],
        specular_range: [0.0; 3],
        envmap_min: 0.0,
        envmap_range: 0.0,
        ..Default::default()
    });
    let mut common_16 = None;
    for shading in [
        Shading::Prelit,
        Shading::WorldNormal(normal),
        Shading::WorldNormal(reflected),
        Shading::WorldWindow(window),
        Shading::SkyLayer(sky),
        Shading::Glossy(glossy),
    ] {
        let handle = mesh(&mut r, texture, shading);
        let instances = [Instance::new(handle, Mat4::IDENTITY)];
        let counts = r.resource_counts();
        let mut images = Vec::new();
        for filtering in [
            TextureFiltering::Trilinear,
            TextureFiltering::Anisotropic8,
            TextureFiltering::Anisotropic16,
            TextureFiltering::Trilinear,
        ] {
            let wanted = GraphicsSettings { texture_filtering: filtering, ..Default::default() };
            let result = r.apply_graphics(&wanted);
            assert!(result.is_exact(), "{:?}", result.downgrades);
            assert_eq!(r.graphics().texture_filtering, filtering);
            r.render(&frame(), &instances).unwrap();
            images.push(r.read_output().unwrap().unwrap());
            assert_eq!(r.resource_counts(), counts, "filtering preserves live texture/mesh counts");
        }
        assert_ne!(images[0], images[2], "{shading:?}: retained sampler must change pixels");
        assert_eq!(images[0], images[3], "{shading:?}: returning to trilinear restores the same image");
        if shading == Shading::Prelit {
            common_16 = Some(images[2].clone());
        }
        r.destroy_mesh(handle);
    }
    r.destroy_normal_material(normal);
    r.destroy_normal_material(reflected);
    r.destroy_window_material(window);
    r.destroy_sky_material(sky);
    r.destroy_glossy_material(glossy);
    for handle in [texture, flat, black] {
        r.destroy_texture(handle);
    }
    assert_eq!(r.resource_counts(), original_counts);
    // Rebinding after release must neither resurrect retired materials nor break future uploads.
    r.set_texture_filtering(TextureFiltering::Anisotropic16);
    let texture = r.create_texture(&TextureDesc {
        label: "late striped surface",
        width: 128,
        height: 128,
        format: PixelFormat::Rgba8,
        mips: levels.iter().map(Vec::as_slice).collect(),
    });
    let mesh = mesh(&mut r, texture, Shading::Prelit);
    r.render(&frame(), &[Instance::new(mesh, Mat4::IDENTITY)]).unwrap();
    assert_eq!(r.read_output().unwrap().unwrap(), common_16.unwrap(), "future uploads inherit active filtering");
    r.destroy_mesh(mesh);
    r.destroy_texture(texture);
    assert_eq!(r.resource_counts(), original_counts);
}

#[test]
#[ignore = "needs a GPU"]
fn texture_scroll_changes_pixels_and_survives_sampler_rebuild() {
    let _guard = test_support::serial();
    let mut r = Renderer::headless((64, 64), test_support::options()).expect("selected GPU required");
    let levels = striped_mips();
    let texture = r.create_texture(&TextureDesc {
        label: "scroll stripes",
        width: 128,
        height: 128,
        format: PixelFormat::Rgba8,
        mips: levels.iter().map(Vec::as_slice).collect(),
    });
    let mesh = mesh(&mut r, texture, Shading::Prelit);
    let instances = [Instance::new(mesh, Mat4::IDENTITY)];
    r.set_texture_filtering(TextureFiltering::Anisotropic8);
    r.render(&frame(), &instances).unwrap();
    let before = r.read_output().unwrap().unwrap();
    r.set_texture_uv_offset(texture, [0.0, 0.0625]);
    r.render(&frame(), &instances).unwrap();
    let shifted = r.read_output().unwrap().unwrap();
    assert_ne!(before, shifted);
    r.set_texture_filtering(TextureFiltering::Trilinear);
    r.set_texture_filtering(TextureFiltering::Anisotropic8);
    r.render(&frame(), &instances).unwrap();
    assert_eq!(shifted, r.read_output().unwrap().unwrap());
    r.set_texture_uv_offset(texture, [0.0, 0.0]);
    r.render(&frame(), &instances).unwrap();
    let unshifted = r.read_output().unwrap().unwrap();
    r.set_texture_uv_offset(texture, [0.0, 1.0]);
    r.render(&frame(), &instances).unwrap();
    assert_eq!(unshifted, r.read_output().unwrap().unwrap());
    r.destroy_mesh(mesh);
    r.destroy_texture(texture);
}
