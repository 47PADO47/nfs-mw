//! Coloured body chrome must retain selected colour and composite skin detail.
use super::*;
use blackbox_gfx::{PixelFormat, SurfaceKind, TextureDesc};

fn render(r: &mut Renderer, kind: SurfaceKind, colour: [u8; 4], stripes: bool, diffuse: f32) -> Vec<u8> {
    let mut pixels = Vec::new();
    for _y in 0..4 {
        for x in 0..4 {
            pixels.extend_from_slice(&match stripes && x < 2 {
                true => [0, 0, 0, 255],
                false => colour,
            });
        }
    }
    let texture = r.create_texture(&TextureDesc {
        label: "chrome composite colour",
        width: 4,
        height: 4,
        format: PixelFormat::Rgba8,
        mips: vec![&pixels],
    });
    let desc = GlossyMaterial {
        diffuse_min: [diffuse, diffuse, diffuse, 1.0],
        diffuse_range: [0.0; 4],
        surface: Some(GlossySurface { kind, roughness: 0.12, coat: 0.0 }),
        ..material(0.12)
    };
    let handle = r.create_glossy_material(&desc);
    let mut mesh = MeshBuilder::new();
    mesh.state(Some(texture), BlendMode::Opaque, Shading::Glossy(handle));
    mesh.flat_quad(
        [Vec3::new(-5.0, -5.0, 0.0), Vec3::new(5.0, -5.0, 0.0), Vec3::new(5.0, 5.0, 0.0), Vec3::new(-5.0, 5.0, 0.0)],
        [128, 128, 128, 255],
    );
    let mesh = mesh.finish().upload(r, "chrome composite receiver");
    let image = r.capture(96, 96, &frame(), &[Instance::new(mesh, Mat4::IDENTITY)]).unwrap();
    r.destroy_mesh(mesh);
    r.destroy_texture(texture);
    r.destroy_glossy_material(handle);
    image
}
#[test]
#[ignore = "requires a GPU"]
fn body_chrome_keeps_colour_when_authored_diffuse_is_black_and_increases_vinyl_contrast() {
    let _serial = test_support::serial();
    let mut r = test_support::headless(SIZE).expect("GPU required");
    rig(&mut r, 0.0);
    let face = [180, 180, 180, 255];
    r.set_environment(blackbox_gfx::Environment::Faces { size: 1, faces: [&face; 6] });
    let red = render(&mut r, SurfaceKind::TintedChrome, [220, 20, 15, 255], false, 0.0);
    let blue = render(&mut r, SurfaceKind::TintedChrome, [15, 20, 220, 255], false, 0.0);
    let p = (48 * 96 + 48) * 4;
    assert!(red[p] > red[p + 2] + 25, "selected red must survive zero authored diffuse");
    assert!(blue[p + 2] > blue[p] + 25, "selected blue must survive zero authored diffuse");
    let old = render(&mut r, SurfaceKind::Conductor, [255; 4], true, 1.0);
    let new = render(&mut r, SurfaceKind::TintedChrome, [255; 4], true, 1.0);
    let contrast = |image: &[u8]| {
        let values: Vec<_> = (24..72).map(|x| image[(48 * 96 + x) * 4]).collect();
        i16::from(*values.iter().max().unwrap()) - i16::from(*values.iter().min().unwrap())
    };
    assert!(
        contrast(&new) > contrast(&old) + 10,
        "composite pattern contrast must improve: {} -> {}",
        contrast(&old),
        contrast(&new)
    );
    let dark_face = [0, 0, 0, 255];
    r.set_environment(blackbox_gfx::Environment::Faces { size: 1, faces: [&dark_face; 6] });
    let dark = render(&mut r, SurfaceKind::TintedChrome, [220, 20, 15, 255], false, 0.0);
    assert!(dark[p] < red[p] / 3, "body chrome must remain reflective rather than inventing flat diffuse colour");
}
