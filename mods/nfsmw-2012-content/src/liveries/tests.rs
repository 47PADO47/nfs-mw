use super::*;

/// A unit square on the z = 0 plane whose texture coordinates are its x and y.
fn square() -> PaintMap {
    let corner = |x: f32, y: f32| ([x, y, 0.0], [x, y]);
    let triangle = |a, b, c| {
        let ((pa, ua), (pb, ub), (pc, uc)) = (a, b, c);
        Triangle { corners: [pa, pb, pc], uv: [ua, ub, uc] }
    };
    PaintMap::new(vec![
        triangle(corner(0.0, 0.0), corner(1.0, 0.0), corner(1.0, 1.0)),
        triangle(corner(0.0, 0.0), corner(1.0, 1.0), corner(0.0, 1.0)),
    ])
}

fn vertex(position: [f32; 3]) -> Vertex {
    Vertex { position, normal: [0.0, 0.0, 1.0], color_bgra: [255; 4], uv: [9.0, 9.0] }
}

#[test]
fn a_point_over_the_surface_takes_the_coordinates_beneath_it() {
    let uv = square().uv_at([0.25, 0.75, 0.3]).unwrap();
    assert!((uv[0] - 0.25).abs() < 1e-5 && (uv[1] - 0.75).abs() < 1e-5, "{uv:?}");
}

#[test]
fn a_point_off_the_edge_takes_the_nearest_edge_point() {
    let uv = square().uv_at([1.5, 0.5, 0.0]).unwrap();
    assert!((uv[0] - 1.0).abs() < 1e-5 && (uv[1] - 0.5).abs() < 1e-5, "{uv:?}");
}

#[test]
fn a_point_far_from_every_surface_has_no_coordinates() {
    assert_eq!(square().uv_at([0.5, 0.5, 20.0]), None);
}

#[test]
fn the_nearest_of_two_layers_wins() {
    let flat = |z: f32, uv: [f32; 2]| Triangle { corners: [[0.0, 0.0, z], [1.0, 0.0, z], [0.0, 1.0, z]], uv: [uv; 3] };
    let map = PaintMap::new(vec![flat(0.0, [0.1, 0.1]), flat(0.5, [0.9, 0.9])]);
    let near = |uv: Option<[f32; 2]>, want: f32| uv.is_some_and(|uv| uv.iter().all(|c| (c - want).abs() < 1e-5));
    assert!(near(map.uv_at([0.2, 0.2, 0.4]), 0.9));
    assert!(near(map.uv_at([0.2, 0.2, 0.1]), 0.1));
}

#[test]
fn triangles_without_area_are_not_indexed() {
    let line = Triangle { corners: [[0.0; 3], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]], uv: [[0.0; 2]; 3] };
    assert!(PaintMap::new(vec![line]).is_empty());
}

#[test]
fn retargeting_gives_every_vertex_the_coordinates_of_the_surface_under_it() {
    let vertices = [vertex([0.1, 0.1, 0.05]), vertex([0.2, 0.1, 0.05]), vertex([0.1, 0.2, 0.05])];
    let (out, indices) = square().retarget(&vertices, &[0, 1, 2]).unwrap();
    assert_eq!(indices, vec![0, 1, 2]);
    assert!((out[1].uv[0] - 0.2).abs() < 1e-5 && (out[1].uv[1] - 0.1).abs() < 1e-5, "{:?}", out[1].uv);
    assert_eq!(out[0].normal, vertices[0].normal);
}

#[test]
fn a_triangle_across_a_seam_of_the_clone_takes_one_texel() {
    // Two surfaces whose layouts are far apart: a triangle with corners over both would smear the picture.
    let island =
        |x: f32, uv: [f32; 2]| Triangle { corners: [[x, 0.0, 0.0], [x + 0.5, 0.0, 0.0], [x, 0.5, 0.0]], uv: [uv; 3] };
    let map = PaintMap::new(vec![island(0.0, [0.1, 0.1]), island(0.6, [0.9, 0.9])]);
    let vertices = [vertex([0.05, 0.05, 0.0]), vertex([0.7, 0.05, 0.0]), vertex([0.05, 0.3, 0.0])];
    let (out, indices) = map.retarget(&vertices, &[0, 1, 2]).unwrap();
    let first = out[usize::from(indices[0])].uv;
    assert!(indices.iter().all(|&at| out[usize::from(at)].uv == first), "{out:?}");
}

#[test]
fn a_vertex_is_split_where_its_triangles_disagree() {
    // Vertex 0 sits on the seam: the first triangle is over island one, the second is flattened onto island two.
    let island =
        |x: f32, uv: [f32; 2]| Triangle { corners: [[x, 0.0, 0.0], [x + 0.5, 0.0, 0.0], [x, 0.5, 0.0]], uv: [uv; 3] };
    let map = PaintMap::new(vec![island(0.0, [0.1, 0.1]), island(0.6, [0.9, 0.9])]);
    let vertices = [
        vertex([0.2, 0.05, 0.0]),
        vertex([0.3, 0.05, 0.0]),
        vertex([0.2, 0.2, 0.0]),
        vertex([0.7, 0.05, 0.0]),
        vertex([0.8, 0.05, 0.0]),
    ];
    let (out, indices) = map.retarget(&vertices, &[0, 1, 2, 0, 3, 4]).unwrap();
    assert_eq!(indices.len(), 6);
    assert!(out.len() >= 5);
    assert_ne!(out[usize::from(indices[0])].uv, out[usize::from(indices[3])].uv);
}
