use crate::build::{BuildError, MAX_COMPONENT, Placement, section, yaw_rows};

fn near(a: [f32; 3], b: [f32; 3]) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-5)
}

fn placement(position: [f32; 3], rows: [[f32; 3]; 3]) -> Placement {
    Placement {
        name: "MON_ARCH_A".to_owned(),
        solids: [0xAABB_CCDD, 0xAABB_CCDD, 0, 0],
        local_min: [-1.0, -2.0, 0.0],
        local_max: [1.0, 2.0, 3.0],
        radius: 3.7,
        position,
        rows,
        exclude_flags: 0x60,
    }
}

#[test]
fn a_yaw_of_ninety_degrees_turns_the_x_axis_to_plus_y() {
    let rows = yaw_rows(90.0, 1.0);
    assert!(near(rows[0], [0.0, 1.0, 0.0]));
    assert!(near(rows[1], [-1.0, 0.0, 0.0]));
    assert!(near(rows[2], [0.0, 0.0, 1.0]));
}

#[test]
fn a_scale_multiplies_every_row() {
    let rows = yaw_rows(0.0, 2.0);
    assert_eq!(rows, [[2.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 2.0]]);
}

#[test]
fn the_origin_of_an_instance_is_its_position() {
    let built = section(1100, &[placement([10.0, 20.0, 30.0], yaw_rows(90.0, 1.0))]).unwrap();
    let columns = built.instances[0].matrix_columns();
    assert_eq!([columns[3][0], columns[3][1], columns[3][2]], [10.0, 20.0, 30.0]);
    assert_eq!(built.section_number, 1100);
}

#[test]
fn the_instance_box_is_the_local_box_turned_by_the_rows() {
    // Yaw 90: world = (-y, x, z) + position. The local box spans x -1..1, y -2..2, z 0..3.
    let built = section(1100, &[placement([10.0, 20.0, 30.0], yaw_rows(90.0, 1.0))]).unwrap();
    let instance = &built.instances[0];
    assert!(near(instance.bbox_min, [8.0, 19.0, 30.0]), "{:?}", instance.bbox_min);
    assert!(near(instance.bbox_max, [12.0, 21.0, 33.0]), "{:?}", instance.bbox_max);
}

#[test]
fn each_placement_gets_its_own_info_at_the_same_index() {
    let placements = [placement([0.0; 3], yaw_rows(0.0, 1.0)), placement([5.0; 3], yaw_rows(0.0, 1.0))];
    let built = section(1100, &placements).unwrap();
    assert_eq!(built.infos.len(), 2);
    assert_eq!(built.instances[1].info_index, 1);
    assert_eq!(built.info_of(&built.instances[1]).unwrap().name, "MON_ARCH_A");
    assert_eq!(built.infos[0].solid_keys, vec![0xAABB_CCDD, 0xAABB_CCDD, 0, 0]);
    assert_eq!(built.infos[0].radius, 3.7);
}

#[test]
fn a_component_beyond_the_format_is_refused_with_its_placement() {
    let too_big = [[MAX_COMPONENT + 0.5, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let placements = [placement([0.0; 3], yaw_rows(0.0, 1.0)), placement([0.0; 3], too_big)];
    assert_eq!(section(1100, &placements), Err(BuildError::Rotation { placement: 1 }));
}

#[test]
fn a_position_that_is_not_finite_is_refused() {
    let placements = [placement([f32::NAN, 0.0, 0.0], yaw_rows(0.0, 1.0))];
    assert_eq!(section(1100, &placements), Err(BuildError::Position { placement: 0 }));
}
