//! The cop's corridor and separation sets (spec `ai-pursuit-obstacle-selection.md`, avoidable sets).

use super::*;

fn at(x: f32) -> Body {
    Body { position: Vec3::X * x, velocity: Vec3::ZERO, forward: Vec2::Y, half_width: 1.0, half_length: 2.0 }
}

#[test]
fn a_cop_out_of_formation_cuts_round_the_target_and_everyone_and_keeps_its_distance_from_all() {
    let (mut corridor, mut separation) = (Vec::new(), Vec::new());
    avoid_sets(at(0.0), false, [(at(1.0), true), (at(2.0), true)], &mut corridor, &mut separation);
    assert_eq!(corridor, [at(0.0), at(1.0), at(2.0)]);
    assert_eq!(separation, corridor);
}

#[test]
fn a_cop_in_formation_ignores_the_target_and_its_formation_mates_for_its_corridor_only() {
    let (mut corridor, mut separation) = (Vec::new(), Vec::new());
    // at(1) is a slotted mate (flagged not to cut), at(2) is traffic.
    avoid_sets(at(0.0), true, [(at(1.0), false), (at(2.0), true)], &mut corridor, &mut separation);
    assert_eq!(corridor, [at(2.0)]);
    assert_eq!(separation, [at(0.0), at(1.0), at(2.0)]);
}
