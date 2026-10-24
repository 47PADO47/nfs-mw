//! Trail speed curvature: mirrored apex angles and signed, length-weighted road bends.

use glam::{Vec2, Vec3};

use crate::{Cookie, Occlusion, trail_curvature};

fn cookie(z: f32, curvature: f32, length: f32) -> Cookie {
    let mut cookie = Cookie::new(Vec3::Z * z, Vec2::new(-2.0, z), Vec2::new(2.0, z), curvature, 0);
    cookie.length = length;
    cookie
}

fn view(current: Cookie) -> Occlusion {
    Occlusion {
        current,
        current_index: 0,
        out_of_bounds: 0.0,
        road: 0,
        avoidable: 0,
        from_behind: false,
        apex: Vec3::Z * 40.0,
        apex_width: 4.0,
        position: Vec3::Z * 50.0,
        trail_speed: 0.0,
        closing_speed: 0.0,
    }
}

#[test]
fn trail_curvature_mirrored_obtuse_apex_angles_have_the_same_speed_limit() {
    let cookies = [cookie(0.0, 0.0, 0.0), cookie(30.0, 0.0, 30.0), cookie(60.0, 0.0, 30.0)];
    let mut occlusion = view(cookies[0]);
    occlusion.road = 1;
    let angle = 140.0_f32.to_radians();
    let expected = angle / 50.0;
    for side in [-1.0, 1.0] {
        let direction = Vec3::new(side * angle.sin(), 0.0, angle.cos());
        let cursor = occlusion.apex + direction * 100.0;
        let actual = trail_curvature(&cookies, 0, Vec3::ZERO, &occlusion, cursor, 30.0);
        assert!(
            (actual - expected).abs() < 1e-6,
            "mirrored side {side}: expected unsigned obtuse curvature {expected}, got {actual}"
        );
    }
}

#[test]
fn trail_curvature_s_bends_average_adjacent_signed_samples_before_taking_absolute_value() {
    let cookies = [cookie(0.0, 0.02, 0.0), cookie(10.0, -0.02, 10.0), cookie(40.0, 0.004, 30.0)];
    let occlusion = view(cookies[0]);
    // Clamp to [0.01, -0.01, 0.004], then use adjacent signed averages and the later sample's length.
    // The first pair cancels; the second contributes -0.003 * 30 m. Take abs after dividing by 40 m.
    let expected = 0.00225;
    let actual = trail_curvature(&cookies, 0, Vec3::ZERO, &occlusion, Vec3::Z * 60.0, 30.0);
    assert!(
        (actual - expected).abs() < 1e-7,
        "signed trapezoidal curvature must preserve cancellation and length weights: {actual}"
    );
}
