//! The terrain of a section on authored triangles: no game data.

use super::*;

/// A flat square of side `side` from the origin at height `z`, as two triangles.
fn square(side: f32, z: f32) -> Vec<Triangle> {
    vec![[[0.0, 0.0, z], [side, 0.0, z], [side, side, z]], [[0.0, 0.0, z], [side, side, z], [0.0, side, z]]]
}

#[test]
fn flat_terrain_has_one_height_everywhere_inside_it() {
    let terrain = Terrain::from_triangles(square(10.0, 3.0)).unwrap();
    for at in [[1.0, 1.0], [9.0, 2.0], [5.0, 5.0], [0.0, 0.0]] {
        assert_eq!(terrain.surface(at, 3.0, 5.0, 5.0), Some(3.0), "{at:?}");
    }
    assert_eq!(terrain.surface([11.0, 5.0], 3.0, 5.0, 5.0), None, "outside the triangles");
    assert_eq!(terrain.surface([-40.0, 5.0], 3.0, 5.0, 5.0), None, "far outside the grid");
}

#[test]
fn a_slope_is_read_between_its_corners() {
    let slope = vec![[[0.0, 0.0, 0.0], [10.0, 0.0, 10.0], [0.0, 10.0, 0.0]]];
    let terrain = Terrain::from_triangles(slope).unwrap();
    let z = terrain.surface([5.0, 1.0], 5.0, 20.0, 20.0).unwrap();
    assert!((z - 5.0).abs() < 1e-4, "halfway up the slope: {z}");
}

#[test]
fn the_surface_nearest_the_height_asked_for_is_the_ground() {
    // A road at 0 and a bridge deck over it at 6.
    let mut both = square(10.0, 0.0);
    both.extend(square(10.0, 6.0));
    let terrain = Terrain::from_triangles(both).unwrap();
    assert_eq!(terrain.surface([5.0, 4.0], 0.5, 3.0, 3.0), Some(0.0), "a tree at the road's height stands on the road");
    assert_eq!(terrain.surface([5.0, 4.0], 5.0, 3.0, 3.0), Some(6.0), "one at the deck's height stands on the deck");
    assert_eq!(terrain.surface([5.0, 4.0], 3.0, 3.0, 3.0), Some(0.0), "equally near: the lower");
}

#[test]
fn a_surface_outside_the_window_is_not_the_ground() {
    let terrain = Terrain::from_triangles(square(10.0, 0.0)).unwrap();
    assert_eq!(
        terrain.surface([5.0, 4.0], 20.0, 12.0, 3.0),
        None,
        "20 m above the ground is more than the window below"
    );
    assert_eq!(terrain.surface([5.0, 4.0], -5.0, 3.0, 3.0), None, "5 m under it is more than the window above");
    assert_eq!(terrain.surface([5.0, 4.0], 8.0, 12.0, 3.0), Some(0.0));
}

#[test]
fn a_big_triangle_is_found_from_every_cell_it_covers() {
    let big = vec![[[0.0, 0.0, 1.0], [100.0, 0.0, 1.0], [0.0, 100.0, 1.0]]];
    let terrain = Terrain::from_triangles(big).unwrap();
    assert_eq!(terrain.surface([80.0, 10.0], 1.0, 2.0, 2.0), Some(1.0));
    assert_eq!(terrain.surface([10.0, 80.0], 1.0, 2.0, 2.0), Some(1.0));
    assert_eq!(terrain.surface([80.0, 80.0], 1.0, 2.0, 2.0), None, "past the hypotenuse");
}

#[test]
fn no_triangles_or_a_broken_one_is_no_terrain() {
    assert!(Terrain::from_triangles(Vec::new()).is_none());
    let broken = vec![[[0.0, 0.0, f32::NAN], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]];
    assert!(Terrain::from_triangles(broken).is_none());
}

#[test]
fn only_the_solids_named_trn_are_terrain() {
    assert!(is_terrain("TRN_CONDOROWBUILD_02_CHOP_O27_R0"));
    assert!(is_terrain("trn_hill"));
    assert!(!is_terrain("SHD_CONDOROWHILL_01A_CHOP_R2_0"));
    assert!(!is_terrain("XT_CT_TREELINE1_1B_00"));
    assert!(!is_terrain("TR"));
}
