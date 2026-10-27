//! Emission remains role-specific without changing neutral details.
use super::*;
use blackbox_gfx::{PixelFormat, SurfaceKind, TextureDesc};

fn render(r: &mut Renderer, kind: SurfaceKind, color: [u8; 4]) -> [u8; 3] {
    let pixels = color.repeat(16);
    let texture = r.create_texture(&TextureDesc {
        label: "lamp test",
        width: 4,
        height: 4,
        format: PixelFormat::Rgba8,
        mips: vec![&pixels],
    });
    let desc = GlossyMaterial {
        surface: Some(GlossySurface { kind, roughness: 0.2, coat: 0.0 }),
        envmap_min: 0.0,
        specular_min: [0.0; 3],
        ..material(0.2)
    };
    let handle = r.create_glossy_material(&desc);
    let mut mesh = MeshBuilder::new();
    mesh.state(Some(texture), BlendMode::Opaque, Shading::Glossy(handle));
    mesh.flat_quad(
        [Vec3::new(-5.0, -5.0, 0.0), Vec3::new(5.0, -5.0, 0.0), Vec3::new(5.0, 5.0, 0.0), Vec3::new(-5.0, 5.0, 0.0)],
        [128, 128, 128, 255],
    );
    let mesh = mesh.finish().upload(r, "lamp receiver");
    let image = r.capture(96, 96, &frame(), &[Instance::new(mesh, Mat4::IDENTITY)]).unwrap();
    let at = (48 * 96 + 48) * 4;
    let result = [image[at], image[at + 1], image[at + 2]];
    r.destroy_mesh(mesh);
    r.destroy_texture(texture);
    r.destroy_glossy_material(handle);
    result
}

#[test]
#[ignore = "needs a GPU"]
fn red_brake_emission_does_not_turn_neutral_details_red() {
    let _serial = test_support::serial();
    let mut r = test_support::headless(SIZE).expect("GPU required");
    rig(&mut r, 0.0);
    let black = [0, 0, 0, 255];
    r.set_environment(blackbox_gfx::Environment::Faces { size: 1, faces: [&black; 6] });
    let brake = render(&mut r, SurfaceKind::EmissiveRed, [255, 150, 30, 255]);
    assert!(u16::from(brake[0]) > u16::from(brake[1]) * 2 && brake[0] > 40, "brake: {brake:?}");
    let neutral = render(&mut r, SurfaceKind::EmissiveRed, [255, 255, 255, 255]);
    assert_eq!(neutral, [0, 0, 0]);
    let head = render(&mut r, SurfaceKind::Emissive, [255, 255, 255, 255]);
    assert!(head.iter().all(|&v| v > 100), "head: {head:?}");
}
