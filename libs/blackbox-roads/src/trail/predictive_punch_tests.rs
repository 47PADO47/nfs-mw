use super::*;

fn body(z: f32, speed: f32) -> Body {
    Body { position: Vec3::Z * z, velocity: Vec3::Z * speed, forward: Vec2::Y, half_width: 1.0, half_length: 2.0 }
}

fn straight_cookies() -> Vec<Cookie> {
    (0..=30)
        .map(|i| {
            let z = i as f32 * 3.0;
            Cookie::new(Vec3::Z * z, Vec2::new(-4.0, z), Vec2::new(4.0, z), 0.0, 0)
        })
        .collect()
}

#[test]
fn moving_avoidable_cuts_begin_near_the_predicted_encounter_not_the_current_body() {
    let mut cookies = straight_cookies();
    let blocker = punch_avoidables(&mut cookies, &body(0.0, 85.0), [body(12.0, 79.0)], false, 1.0);
    assert!(blocker.is_some(), "the slower car is a real future encounter");
    let first = cookies.iter().find(|cookie| cookie.flags & CUT != 0).expect("the encounter must cut the trail");
    // Each extent is 3.5m. The nose gap is 5m and closes at6m/s;
    // the other car moves about66m before the predicted encounter.
    let predicted = 12.0 + 79.0 * (5.0 / 6.0) - 7.0;
    assert!(
        (first.centre.z - predicted).abs() <= 3.0,
        "cut the nearest predicted encounter cookie, without a cut beside the current body: first={first:?} predicted={predicted}"
    );
    assert!(cookies.iter().filter(|cookie| cookie.centre.z < 50.0).all(|cookie| cookie.flags & CUT == 0));
}

#[test]
fn a_stationary_avoidable_still_cuts_the_nearby_road() {
    let mut cookies = straight_cookies();
    assert!(punch_avoidables(&mut cookies, &body(0.0, 20.0), [body(12.0, 0.0)], false, 1.0).is_some());
    let first = cookies.iter().find(|cookie| cookie.flags & CUT != 0).unwrap();
    assert!(first.centre.z <= 12.0, "a stationary blocker must not be moved far ahead: {first:?}");
}

#[test]
fn opposing_vehicles_predict_noses_along_the_road_instead_of_the_other_body_heading() {
    let mut cookies = straight_cookies();
    let other = Body { forward: -Vec2::Y, ..body(50.0, -40.0) };
    assert!(punch_avoidables(&mut cookies, &body(0.0, 40.0), [other], false, 1.0).is_some());
    let first = cookies.iter().find(|cookie| cookie.flags & CUT != 0).unwrap();
    let predicted = 50.0 - 40.0 * (43.0 / 80.0) - 7.0;
    assert!((first.centre.z - predicted).abs() <= 3.0, "{first:?}, expected near{predicted}");
}

#[test]
fn equal_velocity_and_a_meeting_beyond_the_recorded_trail_do_not_create_a_nearby_cut() {
    for distance in [5.0, 12.0] {
        let mut cookies = straight_cookies();
        punch_avoidables(&mut cookies, &body(0.0, 85.0), [body(distance, 85.0)], false, 1.0);
        assert!(cookies.iter().all(|cookie| cookie.flags & CUT == 0));
    }
    let mut cookies = straight_cookies();
    punch_avoidables(&mut cookies, &body(0.0, 85.0), [body(22.0, 79.0)], false, 1.0);
    assert!(cookies.iter().all(|cookie| cookie.flags & CUT == 0), "the predicted meeting lies beyond this trail");
}

#[test]
fn a_nearby_closing_vehicle_retains_the_inside_flag_on_its_predicted_cut() {
    let mut cookies = straight_cookies();
    punch_avoidables(&mut cookies, &body(0.0, 85.0), [body(5.0, 79.0)], false, 1.0);
    let cuts: Vec<_> = cookies.iter().filter(|cookie| cookie.flags & CUT != 0).collect();
    assert!(!cuts.is_empty());
    assert!(cuts.iter().all(|cookie| cookie.flags & CUT_BEHIND != 0));
    assert!(cuts[0].centre.z > 50.0);
}

#[test]
fn an_avoidable_on_another_road_height_is_ignored() {
    let mut cookies = straight_cookies();
    let other = Body { position: Vec3::new(0.0, 5.01, 12.0), ..body(12.0, 79.0) };
    assert!(punch_avoidables(&mut cookies, &body(0.0, 85.0), [other], false, 1.0).is_none());
    assert!(cookies.iter().all(|cookie| cookie.flags & CUT == 0));
}

#[test]
fn predicted_points_select_a_forward_crossing_without_clamping_to_the_trail_end() {
    let cookies = straight_cookies();
    assert_eq!(closest_cookie_ahead(&cookies, Vec2::new(0.0, -1.0)), Some(0));
    assert_eq!(closest_cookie_ahead(&cookies, Vec2::new(0.0, 70.1)), Some(24));
    assert_eq!(closest_cookie_ahead(&cookies, Vec2::new(0.0, 91.0)), None);
}

#[test]
fn civilian_queue_keeps_slowly_closing_blockers_beyond_three_seconds() {
    let mut cookies = straight_cookies();
    // Keep the predicted encounter off an exact cookie boundary so this isolates eligibility.
    assert!(punch_avoidables(&mut cookies, &body(0.0, 0.5), [body(10.1, 0.0)], true, 1.0).is_some());
    assert!(cookies.iter().any(|cookie| cookie.flags & CUT != 0));

    let mut other_navigation = straight_cookies();
    assert!(punch_avoidables(&mut other_navigation, &body(0.0, 0.5), [body(10.1, 0.0)], false, 1.0).is_none());
    assert!(other_navigation.iter().all(|cookie| cookie.flags & CUT == 0));
}

#[test]
fn civilian_queue_retains_the_time_cutoff_for_inside_obstacles() {
    let mut cookies = straight_cookies();
    punch_avoidables(&mut cookies, &body(0.0, 0.0), [body(-1.0, 0.0)], true, 1.0);
    assert!(cookies.iter().all(|cookie| cookie.flags & CUT == 0));
}

#[test]
fn civilian_queue_checks_nonclosing_range_along_the_road_and_includes_the_side_boundary() {
    let mut cookies = straight_cookies();
    let other = Body { position: Vec3::new(3.0, 0.0, 14.0), ..body(14.0, 0.0) };
    assert!(punch_avoidables(&mut cookies, &body(0.0, 0.0), [other], true, 1.0).is_some());
}

#[test]
fn civilian_queue_ignores_nonclosing_bodies_outside_the_native_side_clearance() {
    let mut cookies = straight_cookies();
    let other = Body { position: Vec3::new(3.01, 0.0, 12.0), ..body(12.0, 0.0) };
    punch_avoidables(&mut cookies, &body(0.0, 0.0), [other], true, 1.0);
    assert!(cookies.iter().all(|cookie| cookie.flags & CUT == 0));
}
