//! The position markers of a replacement (the authored lights of a signal head) and a model that hangs below its origin. Authored
//! solids, no game data.

use blackbox_hash::bstring_hash;
use blackbox_modding::content::{FitLimits, SwapFit};
use blackbox_solid::{GroupSource, PositionMarker, Solid, SolidSource, Vertex, read_solids, write_geometry_pack};

use super::fit::{Frame, Prepared, fit_prepared};

fn marker(param: i32, at: [f32; 3]) -> PositionMarker {
    let mut matrix = [0.0; 16];
    matrix[0] = 1.0;
    matrix[5] = 1.0;
    matrix[10] = 1.0;
    matrix[15] = 1.0;
    matrix[12..15].copy_from_slice(&at);
    PositionMarker { name_hash: bstring_hash("LAMP"), int_param: param, float_params: [0.0; 2], matrix }
}

/// A slab from `low` to `high` (two triangles in the plane of the lowest y) with `markers`, and a pivot at `pivot`.
fn slab(name: &str, low: [f32; 3], high: [f32; 3], markers: Vec<PositionMarker>, pivot: [f32; 3]) -> Solid {
    let vertex = |x: f32, z: f32| Vertex {
        position: [x, low[1], z],
        normal: [0.0, -1.0, 0.0],
        color_bgra: [128; 4],
        uv: [0.0, 0.0],
    };
    let vertices =
        vec![vertex(low[0], low[2]), vertex(high[0], low[2]), vertex(high[0], high[2]), vertex(low[0], high[2])];
    let group = GroupSource { effect_id: 0, flags: 0x4000, ..GroupSource::new(vertices, vec![0, 1, 2, 0, 2, 3]) };
    let mut source = SolidSource::new(name, vec![group]);
    source.texture_hashes = vec![bstring_hash("TEX")];
    source.markers = markers;
    source.transform[12..15].copy_from_slice(&pivot);
    // A slab has no depth: give the box some in y so that the model measures a footprint.
    source.bounds_min[1] = low[1] - 0.1;
    source.bounds_max[1] = high[1] + 0.1;
    read_solids(&write_geometry_pack(&[source]).unwrap()).unwrap().pop().unwrap()
}

fn exact(model: &Solid, stock: &Solid) -> Solid {
    let ready = Prepared::new(model).unwrap();
    fit_prepared(&ready, stock, SwapFit::Exact, Frame::Local, &FitLimits::default())
        .unwrap()
        .expect("accepted as it is")
}

#[test]
fn a_model_that_hangs_below_its_origin_is_a_model_with_a_height() {
    let hanging = slab("HEAD", [-0.2, 0.0, -1.4], [0.2, 0.0, -0.1], Vec::new(), [0.0; 3]);
    assert!(Prepared::new(&hanging).is_ok(), "a signal head hangs from its origin: its top is below z = 0");
    let flat = slab("FLAT", [-0.2, 0.0, -0.1], [0.2, 0.0, -0.1], Vec::new(), [0.0; 3]);
    assert!(Prepared::new(&flat).is_err(), "a model of no height is still refused");
}

#[test]
fn a_models_own_markers_are_placed_with_it_and_carry_the_pivot_of_the_solid_it_replaces() {
    let stock =
        slab("STOCK_HEAD", [-0.2, -0.4, -2.4], [0.5, 0.4, 0.1], vec![marker(4, [0.2, 0.0, -0.7])], [10.0, 20.0, 5.0]);
    let model = slab("HEAD", [-0.1, -0.2, -1.4], [0.4, 0.2, -0.1], vec![marker(4, [0.35, 0.0, -0.4])], [0.0; 3]);
    let fitted = exact(&model, &stock);
    assert_eq!(fitted.markers.len(), 1);
    let at = fitted.markers[0].translation();
    assert!((at[0] - 10.35).abs() < 1e-4 && (at[1] - 20.0).abs() < 1e-4 && (at[2] - 4.6).abs() < 1e-4, "{at:?}");
    assert_eq!(fitted.markers[0].int_param, 4);
    assert_eq!(fitted.transform, stock.transform, "the pivot of the replaced solid");
}

#[test]
fn a_model_with_no_markers_keeps_those_of_the_solid_it_replaces() {
    let stock =
        slab("STOCK_HEAD", [-0.2, -0.4, -2.4], [0.5, 0.4, 0.1], vec![marker(3, [0.2, 0.0, -0.7])], [10.0, 20.0, 5.0]);
    let model = slab("HEAD", [-0.1, -0.2, -1.4], [0.4, 0.2, -0.1], Vec::new(), [0.0; 3]);
    let fitted = exact(&model, &stock);
    assert_eq!(fitted.markers, stock.markers);
}
