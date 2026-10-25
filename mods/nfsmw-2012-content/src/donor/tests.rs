use super::*;

fn marker(name: &str, at: [f32; 3]) -> PositionMarker {
    let mut matrix = [0.0; 16];
    matrix[0] = 1.0;
    matrix[5] = 1.0;
    matrix[10] = 1.0;
    matrix[15] = 1.0;
    matrix[12..15].copy_from_slice(&at);
    PositionMarker { name_hash: bstring_hash(name), int_param: 0, float_params: [0.0; 2], matrix }
}

#[test]
fn a_tail_pipe_moves_to_the_rear_face_of_the_new_body() {
    // The clone's pipe is at x = -2.2; the new body ends at x = -2.05 around it, with a bumper further out of reach.
    let body = [[-2.05, 0.4, 0.3], [-2.0, 0.45, 0.35], [-1.0, 0.4, 0.3], [-2.6, 1.5, 0.3]];
    let mut pipe = marker("EXHAUST", [-2.2, 0.4, 0.3]);
    snap(&mut pipe, &body);
    assert_eq!(pipe.translation(), [-2.05, 0.4, 0.3]);
}

#[test]
fn a_spoiler_mount_moves_to_the_upper_face_of_the_new_body() {
    let body = [[-1.6, 0.0, 0.95], [-1.62, 0.1, 1.0], [-1.6, 0.0, 1.9]];
    let mut mount = marker("SPOILER", [-1.6, 0.0, 1.1]);
    snap(&mut mount, &body);
    assert_eq!(mount.translation(), [-1.6, 0.0, 1.0]);
}

#[test]
fn a_marker_with_no_body_near_it_stays_and_other_markers_are_left_alone() {
    let body = [[5.0, 5.0, 5.0]];
    let mut pipe = marker("EXHAUST", [-2.2, 0.4, 0.3]);
    snap(&mut pipe, &body);
    assert_eq!(pipe.translation(), [-2.2, 0.4, 0.3]);
    let mut light = marker("LEFT_HEADLIGHT", [2.0, 0.6, 0.6]);
    snap(&mut light, &[[1.9, 0.6, 0.6]]);
    assert_eq!(light.translation(), [2.0, 0.6, 0.6]);
}

#[test]
fn a_move_never_exceeds_the_limit() {
    let body = [[-2.2 + MAX_SNAP * 2.0, 0.4, 0.3]];
    let mut pipe = marker("EXHAUST", [-2.2, 0.4, 0.3]);
    snap(&mut pipe, &body);
    assert_eq!(pipe.translation()[0], -2.2);
}
