use super::*;

#[test]
fn bend_projection_uses_car_to_apex_and_cursor_to_apex_vectors() {
    for sign in [-1.0, 1.0] {
        let car = Vec2::ZERO;
        let apex = Vec2::new(0.0, 10.0);
        let cursor = Vec2::new(sign * 10.0, 20.0);
        let body = Body {
            position: Vec3::ZERO,
            velocity: Vec3::Z * 60.0,
            forward: Vec2::Y,
            half_width: 1.0,
            half_length: 2.0,
        };
        let cookie = Cookie::new(Vec3::ZERO, Vec2::new(-10.0, 0.0), Vec2::new(10.0, 0.0), 0.0, 0);
        let occlusion = Occlusion {
            current: cookie,
            current_index: 0,
            out_of_bounds: 0.0,
            road: sign as i8,
            avoidable: 0,
            from_behind: false,
            apex: Vec3::new(0.0, 0.0, 10.0),
            apex_width: 20.0,
            position: Vec3::ZERO,
            trail_speed: 0.0,
            closing_speed: 0.0,
        };
        let result = aim_point(car, apex, cursor, &occlusion, &body);
        let expected = Vec3::new(-sign * 3.535_534, 0.0, 11.464_466);
        assert!(result.distance(expected) < 1e-4, "{result:?} != {expected:?}");
    }
}
