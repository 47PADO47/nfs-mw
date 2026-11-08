use super::*;
use blackbox_skeleton::Influences;

#[test]
fn the_rider_files_are_looked_for_in_the_cars_folder_first() {
    assert_eq!(folders("KTM450"), ["CARS/KTM450/RIDER".to_owned(), "RIDER".to_owned()]);
}

#[test]
fn skinned_positions_become_white_vertices_with_the_meshs_texture_coordinates() {
    let source =
        |uv: [f32; 2]| SkinVertex { position: Vec3::ZERO, normal: Vec3::Z, uv, influences: Influences::default() };
    let sources = [source([0.25, 0.5]), source([1.0, 0.0])];
    let positions = [Vec3::new(1.0, 2.0, 3.0), Vec3::new(4.0, 5.0, 6.0)];
    let normals = [Vec3::X, Vec3::Y];
    let mut out = vec![Vertex { position: [9.0; 3], normal: [9.0; 3], color_bgra: [0; 4], uv: [9.0; 2] }; 5];
    fill(&mut out, &sources, &positions, &normals);
    assert_eq!(out.len(), 2, "the old contents are replaced");
    assert_eq!(out[0].position, [1.0, 2.0, 3.0]);
    assert_eq!(out[1].normal, [0.0, 1.0, 0.0]);
    assert_eq!(out[0].uv, [0.25, 0.5]);
    assert_eq!(out[1].color_bgra, [255; 4]);
}

#[test]
fn a_rider_that_is_not_loaded_is_not_stepped_or_drawn() {
    let mut rider = Rider::default();
    assert!(matches!(rider.state, State::Unloaded));
    rider.step(Some(RiderSample::default()), 0.016);
    let pose = CarPose { position: Vec3::ZERO, rotation: glam::Quat::IDENTITY, wheels: [Default::default(); 4] };
    let mut out = Vec::new();
    rider.instances(&pose, &mut out);
    assert!(out.is_empty());
}
