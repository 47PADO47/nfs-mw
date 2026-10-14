use nfsmw_data::car::physics::{AiCollisionReactions, CollisionReaction, CollisionReactions, GoalReaction};

use super::*;

#[test]
fn speedbreaker_collision_override_scales_temporary_mass_and_inertia_and_restores() {
    let mut player = sim();
    let normal = Vec3::NEG_Z;
    let base = player.object_hit(Vec3::Z, normal, false, true);
    let mass = player.body().mass();
    let inertia = player.body().inertia();
    let physics = crate::settings::speedbreaker::PhysicsModifiers::default();
    player.set_speedbreaker_effect(true, &physics);
    let active = player.object_hit(Vec3::Z, normal, false, true);
    assert_eq!(active.mass_scale, 2.0);
    assert_eq!(active.inertia_scale, base.inertia_scale * (2.0 / base.mass_scale));
    assert_eq!(player.body().mass(), mass);
    assert_eq!(player.body().inertia(), inertia);
    player.set_speedbreaker_effect(false, &physics);
    let restored = player.object_hit(Vec3::Z, normal, false, true);
    assert_eq!(restored.mass_scale, base.mass_scale);
    assert_eq!(restored.inertia_scale, base.inertia_scale);
}

pub(super) fn install(sim: &mut CarSim) {
    let reaction = |scale, roll, bias, elasticity| CollisionReaction {
        mass_scale: scale,
        roll_height: roll,
        weight_bias: bias,
        elasticity,
        ..Default::default()
    };
    let profile = Arc::new(CollisionReactions {
        front: reaction(0.6, 0.2, 0.5, 0.25),
        front_side: reaction(0.0, -0.3, 0.0, 0.1),
        rear: reaction(1.2, 0.1, 0.9, 0.0),
        rear_side: reaction(-1.0, 0.0, -0.5, 0.0),
    });
    sim.reactions = Some(Arc::new(AiCollisionReactions { default: Some(profile), goals: vec![] }));
}

#[test]
fn player_impacts_apply_the_ai_profile_only_to_temporary_mass_inertia_and_centre() {
    let mut ai = sim();
    ai.configure_ai();
    install(&mut ai);
    ai.body_mut().place(Vec3::new(10.0, 5.0, 20.0), Quat::from_rotation_y(0.5));
    let mass = ai.body().mass();
    let cog = ai.body().cog();
    let rotation = ai.body().rotation();
    let point = ai.body().position + rotation.z_axis * 2.0;
    let normal = -rotation.z_axis;
    let base = ai.object_hit(point, normal, false, false);
    let hit = ai.object_hit(point, normal, false, true);
    assert_eq!(hit.mass_scale, base.mass_scale * 0.6);
    assert_eq!(hit.inertia_scale, base.inertia_scale * 0.6);
    assert_eq!(hit.restitution, (base.restitution + 0.25).clamp(0.0, 1.0));
    let expected = ai.body().world_cog() + rotation * Vec3::new(0.0, 0.2, 0.5);
    assert!((hit.centre.unwrap() - expected).length() < 0.00001);
    assert_eq!(ai.body().mass(), mass);
    assert_eq!(ai.body().cog(), cog);
    assert!(base.centre.is_none(), "traffic against traffic gets ordinary material");
}

#[test]
fn reaction_boundaries_and_nonpositive_mass_scales_match_original_selection() {
    let mut ai = sim();
    ai.configure_ai();
    install(&mut ai);
    let base = ai.object_hit(Vec3::Z, Vec3::Z, false, false);
    let side = ai.object_hit(Vec3::Z, Vec3::X, false, true);
    assert_eq!(side.mass_scale, base.mass_scale);
    assert_eq!(side.inertia_scale, base.inertia_scale);
    let end = ai.object_hit(Vec3::Z, Vec3::new(0.707, 0.0, 0.707), false, true);
    assert_eq!(end.mass_scale, base.mass_scale * 0.6);
    let rear = ai.object_hit(Vec3::ZERO, Vec3::Z, false, true);
    assert_eq!(rear.mass_scale, base.mass_scale * 1.2);
    let rear_side = ai.object_hit(Vec3::NEG_Z, Vec3::X, false, true);
    assert_eq!(rear_side.mass_scale, base.mass_scale);
    assert_eq!(rear_side.inertia_scale, base.inertia_scale);
}

#[test]
fn actual_player_roll_centre_does_not_depend_on_the_control_device() {
    let mut player = sim();
    install(&mut player);
    player.vehicle.config.steering_device = blackbox_vehicle::steering::SteeringDevice::Ai;
    let point = Vec3::new(0.0, 0.7, 2.0);
    let hit = player.object_hit(point, Vec3::NEG_Z, true, true);
    assert!((hit.centre.unwrap().y - 0.575).abs() < 0.000001);
    assert!(player.object_hit(point, Vec3::NEG_Z, false, true).centre.is_none());
}

#[test]
fn changing_goal_selects_a_profile_and_an_invalid_override_suppresses_it() {
    let mut ai = sim();
    ai.configure_ai();
    install(&mut ai);
    let reactions = Arc::make_mut(ai.reactions.as_mut().unwrap());
    let profile = reactions.default.clone();
    reactions.goals = vec![GoalReaction { goal: 1, reactions: profile }, GoalReaction { goal: 2, reactions: None }];
    ai.collision_goal = Some(1);
    assert!(ai.object_hit(Vec3::Z, Vec3::Z, false, true).centre.is_some());
    ai.collision_goal = Some(2);
    assert!(ai.object_hit(Vec3::Z, Vec3::Z, false, true).centre.is_none());
}
