use nfsmw_data::car::physics::{AiCollisionReactions, CollisionReaction, CollisionReactions, GoalReaction};

use super::*;

#[test]
fn vehicle_source_immobility_initializes_contact_without_retuning_the_body() {
    let mut vehicle = sim();
    let mut spec = vehicle.vehicle.spec().clone();
    spec.body.objects.immobile = true;
    vehicle.vehicle.set_spec(spec);
    let mass = vehicle.body().mass();
    let inertia = vehicle.body().inertia();
    assert!(vehicle.object_hit(Vec3::Z, Vec3::NEG_Z, false, true).immobile);
    assert_eq!(vehicle.body().mass(), mass);
    assert_eq!(vehicle.body().inertia(), inertia);
}

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

#[test]
fn native_pair_damage_uses_normal_callback_and_stored_mass_with_temporary_ai_scaling() {
    let (mut player, mut cop) = (sim(), sim());
    cop.configure_ai();
    cop.collision_role.cop = true;
    let reaction = CollisionReaction { mass_scale: 0.5, elasticity: 0.4, ..Default::default() };
    let profile =
        Arc::new(CollisionReactions { front: reaction, front_side: reaction, rear: reaction, rear_side: reaction });
    cop.reactions = Some(Arc::new(AiCollisionReactions { default: Some(profile), goals: vec![] }));
    player.body_mut().place(Vec3::ZERO, Quat::IDENTITY);
    let separation = cop.half_dimensions().z * 2.0 - 0.1;
    cop.body_mut().place(Vec3::Z * separation, Quat::IDENTITY);
    player.body_mut().linear_velocity = Vec3::new(5.0, 0.0, 20.0);
    let before = cop.body().linear_velocity;
    let mass = cop.body().mass();
    let scale = cop.body().spec().objects.mass_scale * reaction.mass_scale;
    let hit = player.collide_with(&mut cop).expect("approaching bodies overlap");
    let callback = hit.damage[1];
    let expected = 2.0 * hit.impulse / scale;
    assert!((callback.force - expected).abs() < expected * 1e-5);
    assert_eq!(callback.before_velocity, before);
    assert!((callback.closing_speed - 20.0).abs() < 1e-5, "sliding is not part of damage closing speed");
    assert_eq!(callback.normal, -hit.damage[0].normal);
    assert_eq!(callback.arm, cop.body().rotation().transpose() * (hit.point - cop.body().position));
    assert_eq!(hit.damage[0].arm, player.body().rotation().transpose() * (hit.point - player.body().position));
    assert!(callback.other.player && callback.other.vehicle);
    assert_eq!(cop.body().mass(), mass, "temporary collision scaling must not change stored police mass");
    let final_velocity_force = (cop.body().linear_velocity - before).length() * mass * 2.0;
    assert!((final_velocity_force - callback.force).abs() > 1.0, "friction and restitution are excluded from damage");
}

#[test]
#[ignore = "needs installed police mass and collision reaction records"]
fn installed_cop_native_weight_and_goal_reactions_preserve_the_source_physical_body() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR")).unwrap();
    let data = nfsmw_data::car::physics::PhysicsData::load(&dir).unwrap();
    for name in [
        "copmidsize",
        "copghost",
        "copgto",
        "copgtoghost",
        "copsuv",
        "copsuvl",
        "copsuvpatrol",
        "copcross",
        "copsport",
        "copsportghost",
    ] {
        let physics = data.car(name).unwrap();
        let source_mass = data.database().collection("pvehicle", name).unwrap().get_f32("MASS").unwrap();
        assert_eq!(physics.spec.mass, source_mass, "{name}: physical mass must be inherited from its vehicle record");
        let reactions = physics.reactions.as_ref().expect("installed police collision profiles").clone();
        let mut sim = CarSim::new(physics, [0.3; 4]);
        sim.configure_ai();
        let before_mass = sim.body().mass();
        let before_inertia = sim.body().inertia();
        for goal in std::iter::once(None).chain(reactions.goals.iter().map(|entry| Some(entry.goal))) {
            sim.collision_goal = goal;
            let base = sim.object_hit(Vec3::Z, Vec3::NEG_Z, false, false);
            let contact = sim.object_hit(Vec3::Z, Vec3::NEG_Z, false, true);
            let expected_scale = reactions.profile(goal).map_or(1.0, |profile| {
                let scale = profile.front.mass_scale;
                if scale > 0.0 { scale } else { 1.0 }
            });
            assert!((contact.mass_scale - base.mass_scale * expected_scale).abs() < 1e-5, "{name}: goal {goal:?}");
            assert_eq!(sim.body().mass(), before_mass);
            assert_eq!(sim.body().inertia(), before_inertia);
        }
        eprintln!("{name}: native mass={source_mass}, {} authored goal reaction overrides", reactions.goals.len());
    }
}
