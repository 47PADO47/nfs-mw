//! Fast sliding must not bypass the original normal-speed damage gate.

use super::*;

#[test]
fn native_pair_damage_reports_normal_closing_independently_of_fast_sliding() {
    for reverse in [false, true] {
        let (mut a, mut b) = (sim(), sim());
        let distance = a.body().dimension().z + b.body().dimension().z - 0.1;
        a.body_mut().place(Vec3::ZERO, Quat::IDENTITY);
        b.body_mut().place(Vec3::Z * distance, Quat::IDENTITY);
        a.body_mut().linear_velocity = Vec3::new(30.0, 0.0, 0.5);
        let hit = match reverse {
            true => b.collide_with(&mut a),
            false => a.collide_with(&mut b),
        }
        .expect("the shallow approaching contact must still resolve physically");
        assert!(hit.impulse > 0.0);
        for damage in hit.damage {
            assert!((damage.closing_speed - 0.5).abs() < 1e-5, "{damage:?}");
            assert!(damage.closing_speed < 1.0, "sliding does not make the damage callback eligible");
        }
    }
}
