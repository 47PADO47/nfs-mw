//! Multi-car formation integration: one eligible set drives both readiness and the actual commands.

use super::*;
use crate::scenes::world::ai::car::support_drive::{SupportCar, SupportGoal};
use blackbox_pursuit::formation::Formation;

fn world() -> TrafficWorld {
    let mut world = TrafficWorld::new(straight_road(1000.0));
    let mut renderer = free_roam::renderer();
    for at in [Vec3::new(0.0, 0.0, 14.0), Vec3::new(-3.5, 0.0, 0.0), Vec3::new(3.5, 0.0, 0.0)] {
        free_roam::patrol(&mut world, &mut renderer, at);
        let key = world.cars.len() as u32;
        world.cars.last_mut().unwrap().set_entity(key);
    }
    world.player_position = Some(Vec3::ZERO);
    world.start_pursuit(1, vec![wave()], player());
    assert!(world.cars.iter().all(AiCar::is_cop));
    world.police.test_formation(Formation::BoxIn, false);
    world
}

#[test]
fn line_of_sight_filters_both_readiness_and_slot_execution() {
    let world = world();
    let visible = world.formation_layout(Vec3::ZERO, Vec2::Y, &|_, _| true);
    assert_eq!(visible.slots.iter().flatten().count(), 3);
    assert_eq!(
        world
            .formation_gap_input(Vec3::ZERO, space::to_render(Vec3::Z.to_array()), Vec3::ZERO, &|_, _| true)
            .unwrap()
            .0,
        3
    );
    let blocked = world.formation_layout(Vec3::ZERO, Vec2::Y, &|_, _| false);
    assert!(blocked.slots.iter().all(Option::is_none));
    assert_eq!(
        world.formation_gap_input(Vec3::ZERO, space::to_render(Vec3::Z.to_array()), Vec3::ZERO, &|_, _| false),
        None
    );
    assert!(blocked.aims.iter().all(|p| p.y.abs() == 25.0), "excluded cars get the spread grid instead");
}

#[test]
fn support_cannot_occupy_or_arm_a_formation_slot() {
    let mut world = world();
    world.cars[0].set_support(Some(SupportCar { serial: 1, goal: SupportGoal::Chase }));
    let layout = world.formation_layout(Vec3::ZERO, Vec2::Y, &|_, _| true);
    assert_eq!(layout.cops, [1, 2]);
    assert_eq!(layout.slots.iter().flatten().count(), 2);
    let gap =
        world.formation_gap_input(Vec3::ZERO, space::to_render(Vec3::Z.to_array()), Vec3::ZERO, &|_, _| true).unwrap();
    assert_eq!(gap.0, 1, "the support cop standing on the front slot cannot count towards readiness");
}

#[test]
fn attack_slot_identity_survives_target_rotation() {
    let mut world = world();
    let approach = world.formation_layout(Vec3::ZERO, Vec2::Y, &|_, _| true);
    world.police.test_formation(Formation::BoxIn, true);
    let attack = world.formation_layout(Vec3::ZERO, -Vec2::Y, &|_, _| true);
    assert_eq!(attack.slots, approach.slots, "attackers must not change goal record mid-finisher");
}

#[test]
fn collapse_releases_each_goal_when_the_target_drives_away() {
    let world = world();
    let collision = floor();
    let mut aims = Vec::new();
    let target =
        Body { position: Vec3::ZERO, velocity: Vec3::ZERO, forward: Vec2::Y, half_width: 1.0, half_length: 2.0 };
    world.cop_aims(&target, &collision, &mut aims);
    assert!(world.collapse_active());
    assert!(world.cars.iter().all(|car| car.goal_drive().kind == Some(blackbox_pursuit::goals::GoalKind::PullOver)));
    world.cop_aims(&Body { velocity: Vec3::Z * 50.0, ..target }, &collision, &mut aims);
    assert!(!world.collapse_active());
    assert!(world.cars.iter().all(|car| car.goal_drive().kind.is_none()));
}

#[test]
fn sideways_target_motion_drives_both_slot_execution_and_readiness() {
    let mut world = world();
    let positions = [Vec3::new(14.0, 1.0, 0.0), Vec3::new(0.0, 1.0, 3.5), Vec3::new(0.0, 1.0, -3.5)];
    for (car, position) in world.cars.iter_mut().zip(positions) {
        car.sims_mut().0.body_mut().place(position, glam::Quat::IDENTITY);
    }
    let target =
        Body { position: Vec3::ZERO, velocity: Vec3::X * 50.0, forward: Vec2::Y, half_width: 1.0, half_length: 2.0 };
    let expected = world.formation_layout(target.position, Vec2::X, &|_, _| true);
    let mut aims = Vec::new();
    world.cop_aims(&target, &floor(), &mut aims);
    for (&car, expected) in expected.cops.iter().zip(&expected.aims) {
        let aim = aims[car].expect("a visible formation member has a slot");
        assert!((Vec2::new(aim.x, aim.z) - *expected).length() < 0.001);
    }
    let ready = world
        .formation_gap_input(
            target.position,
            space::to_render(Vec3::Z.to_array()),
            space::to_render(target.velocity.to_array()),
            &|_, _| true,
        )
        .unwrap();
    assert_eq!(ready.0, 3, "readiness must use the same moving target frame as slot execution");
}
