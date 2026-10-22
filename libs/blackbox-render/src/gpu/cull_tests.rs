//! GPU checks of back-face culling: which faces a single-sided draw keeps through mirroring instances and views.

use super::{Renderer, test_support};
use blackbox_gfx::*;
use glam::{Mat4, Vec3};

const CLEAR: [f32; 3] = [0.0, 0.0, 1.0];

/// Clip space is the world: the identity projection and view, so a triangle's corners are where they show.
fn frame(view: Mat4) -> FrameParams {
    FrameParams {
        view,
        projection: Projection::Identity,
        camera_position: Vec3::new(0.0, 0.0, 3.0),
        light_dir: Vec3::NEG_Z,
        clear_color: CLEAR,
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: false,
        shadow_tuning: None,
    }
}

/// A quad over the whole picture, its corners counter-clockwise on screen (`ccw`) or clockwise.
fn quad(r: &mut Renderer, ccw: bool, two_sided: bool) -> MeshHandle {
    let vertices = [[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]].map(|[x, y]| Vertex {
        position: [x, y, 0.5],
        normal: [0.0, 0.0, 1.0],
        color_bgra: [0, 255, 0, 255],
        uv: [0.0, 0.0],
    });
    let indices: [u16; 6] = match ccw {
        true => [0, 1, 2, 0, 2, 3],
        false => [0, 2, 1, 0, 3, 2],
    };
    r.create_mesh(&MeshDesc {
        label: "cull quad",
        vertices: &vertices,
        tangents: None,
        secondary_uvs: None,
        indices: &indices,
        draws: vec![DrawRange {
            first_index: 0,
            index_count: 6,
            base_vertex: 0,
            texture: None,
            blend: BlendMode::Opaque,
            shading: Shading::Sky,
            two_sided,
        }],
    })
}

/// Whether the quad covers the middle of the picture (green) rather than the clear colour (blue).
fn shows(r: &mut Renderer, view: Mat4, instance: Instance) -> bool {
    assert!(r.render(&frame(view), &[instance]).unwrap());
    let image = r.read_output().unwrap().unwrap();
    let [red, green, blue, _] = image.pixel(16, 16).unwrap();
    assert!(red < 40, "the middle is the quad or the clear colour, not {:?}", [red, green, blue]);
    green > 150 && blue < 100
}

#[test]
#[ignore = "needs a GPU"]
fn single_sided_draws_keep_their_front_faces_through_mirrors() {
    let _gpu = test_support::serial();
    let Some(mut r) = test_support::headless((32, 32)) else { return };
    let front = quad(&mut r, true, false);
    let back = quad(&mut r, false, false);
    let both = quad(&mut r, false, true);
    let mirror = Mat4::from_scale(Vec3::new(1.0, -1.0, 1.0));
    let at = |mesh, transform| Instance::new(mesh, transform);

    assert!(shows(&mut r, Mat4::IDENTITY, at(front, Mat4::IDENTITY)), "front faces are drawn");
    assert!(!shows(&mut r, Mat4::IDENTITY, at(back, Mat4::IDENTITY)), "back faces are culled");
    assert!(shows(&mut r, Mat4::IDENTITY, at(both, Mat4::IDENTITY)), "two-sided draws keep both faces");

    // A mirroring transform turns the front faces clockwise on screen: they are still the ones kept.
    assert!(shows(&mut r, Mat4::IDENTITY, at(front, mirror)), "a mirrored instance keeps its front faces");
    assert!(!shows(&mut r, Mat4::IDENTITY, at(back, mirror)), "and still culls its back faces");

    // Data with the mirror baked into its vertices: its front faces are the clockwise ones, through any transform.
    assert!(shows(&mut r, Mat4::IDENTITY, at(back, Mat4::IDENTITY).with_flipped_winding(true)));
    assert!(!shows(&mut r, Mat4::IDENTITY, at(front, Mat4::IDENTITY).with_flipped_winding(true)));
    assert!(shows(&mut r, Mat4::IDENTITY, at(back, mirror).with_flipped_winding(true)));
    assert!(!shows(&mut r, Mat4::IDENTITY, at(front, mirror).with_flipped_winding(true)));

    // A mirroring view (a reflection) reverses every triangle on screen, and the kept side with it.
    assert!(shows(&mut r, mirror, at(front, Mat4::IDENTITY)), "a mirrored view keeps the front faces");
    assert!(!shows(&mut r, mirror, at(back, Mat4::IDENTITY)), "and culls the back faces");
    assert!(shows(&mut r, mirror, at(front, mirror)), "two mirrors cancel");
}

#[test]
#[ignore = "needs a GPU"]
fn the_culling_pipelines_are_built_only_once_a_single_sided_draw_exists() {
    let _gpu = test_support::serial();
    let Some(mut r) = test_support::headless((32, 32)) else { return };
    let both = quad(&mut r, true, true);
    assert!(!r.pipelines.culled(), "two-sided meshes need no culling pipelines");
    assert!(shows(&mut r, Mat4::IDENTITY, Instance::new(both, Mat4::IDENTITY)));
    let format = r.targets.color_format();
    let two_sided_only = r.pipelines.count(format);
    let back = quad(&mut r, false, false);
    assert!(r.pipelines.culled());
    assert!(!shows(&mut r, Mat4::IDENTITY, Instance::new(back, Mat4::IDENTITY)));
    assert_eq!(r.pipelines.count(format), 3 * two_sided_only, "every pipeline gains a variant per culled side");
}
