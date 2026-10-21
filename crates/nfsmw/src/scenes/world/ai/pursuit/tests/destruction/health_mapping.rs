//! Actual resolved single-box contacts isolate health/event adapters from the detailed-shape matrix.
use super::*;
use crate::scenes::world::ai::collide::DriverClass;
use crate::scenes::world::events::DamageHit;
use nfsmw_data::car::physics::DAMAGE_ZONES;

const TYPES: [&str; 11] = [
    "copmidsize",
    "copghost",
    "copgto",
    "copgtoghost",
    "copsport",
    "copsportghost",
    "copsporthench",
    "copsuv",
    "copsuvl",
    "copsuvpatrol",
    "copcross",
];

fn one_contact(world: &mut TrafficWorld, striker: &mut CarSim, speed: f32, side: bool) -> DamageHit {
    let at = Vec3::Y * 10.0;
    let cop = world.cars[0].sims_mut().0;
    cop.body_mut().place(at, Quat::IDENTITY);
    cop.body_mut().linear_velocity = Vec3::ZERO;
    cop.body_mut().angular_velocity = Vec3::ZERO;
    let axis = if side { Vec3::X } else { Vec3::Z };
    let cop_half = if side { cop.half_dimensions().x } else { cop.half_dimensions().z };
    let heading = if side { -std::f32::consts::FRAC_PI_2 } else { std::f32::consts::PI };
    let separation = cop_half + striker.half_dimensions().z - 0.05;
    striker.body_mut().place(at + axis * separation, Quat::from_rotation_y(heading));
    striker.body_mut().linear_velocity = -axis * speed;
    striker.body_mut().angular_velocity = Vec3::ZERO;
    world.set_damage_view(at);
    world.collide(Some(striker));
    let events = world.cars[0].sims_mut().0.events.as_mut().unwrap();
    let callbacks = events.pending_hits();
    assert_eq!(callbacks.len(), 1, "dedicated geometry=None box fixture has one physical callback");
    let hit = callbacks[0];
    assert!(hit.other.player && hit.other.vehicle);
    assert!(hit.force > 0.0 && hit.force.is_finite());
    events.tick(0.0);
    hit
}

#[test]
#[ignore = "requires installed cop physics, damage records and sequences"]
fn installed_cop_health_zone_mapping_disables_lamps_without_forcing_all_visual_zones() {
    let (_dir, data) = data();
    let mut renderer = NullBackend::new(crate::settings::test_caps::native());
    for name in TYPES {
        let physics = data.car(name).unwrap();
        let spec = physics.events.as_ref().unwrap().clone();
        // This fixture uses installed mass/damage/rules, with one box instead of the detailed shape set.
        let mut model = cop_model(&mut renderer, name, player());
        model.model.physics = physics;
        model.model.physics.geometry = None;
        let mut world = TrafficWorld::new(cop_road());
        world.add_cop_model(model);
        spawn(&mut world, name);
        let mut striker_physics = data.car("bmwm3gtr").unwrap();
        striker_physics.geometry = None;
        let mut striker = CarSim::new(striker_physics, [0.3; 4]);
        for side in [false, true] {
            if side {
                world.cars.clear();
                spawn(&mut world, name);
            }
            // Calibrate one admitted physical callback; the low force remains below every cop health threshold.
            let probe = one_contact(&mut world, &mut striker, 1.01, side);
            let zone = probe.zone();
            assert_eq!(zone, if side { 3 } else { 0 });
            assert_eq!(world.cars[0].cop_damage(), 0.0);
            let scale = spec.hit_point_scale[zone];
            let force_per_speed = probe.force / 1.01;
            let partial_scaled = (spec.hit_points * 0.2).max(spec.hit_point_threshold * 1.1);
            let speed = partial_scaled * spec.strength / scale / force_per_speed;
            let partial = one_contact(&mut world, &mut striker, speed, side);
            let expected = partial.force / spec.strength * scale / spec.hit_points;
            assert!((world.cars[0].cop_damage() - expected).abs() < 0.0001, "{name} {zone} actual callback health");
            assert!(expected > 0.0 && expected < 1.0);
            assert!(!world.cars[0].is_wrecked());
            assert_eq!(
                world.cars[0].sims_mut().0.lamp_damage() & 7,
                0,
                "partial health does not invent broken headlamps"
            );
            let lethal_speed = spec.hit_points * 1.2 * spec.strength / scale / force_per_speed;
            one_contact(&mut world, &mut striker, lethal_speed, side);
            assert!(world.cars[0].is_wrecked(), "{name} lethal physical callback");
            assert_eq!(world.cars[0].sims_mut().0.lamp_damage(), u32::MAX);
            assert!(!world.cars[0].is_live_police() && !world.cars[0].emergency_lights());
            let levels = *world.cars[0].sims_mut().0.events.as_ref().unwrap().source_levels();
            assert!(levels[zone] > 0 && levels[zone] <= 2);
            assert!(
                levels.iter().enumerate().all(|(i, &level)| i == zone || level == 0),
                "{name}: ordinary lethal contact must not fill all zones: {levels:?}"
            );
            assert!(world.cars[0].take_wreck());
            assert!(!world.cars[0].take_wreck());
            let damage = world.cars[0].cop_damage();
            one_contact(&mut world, &mut striker, lethal_speed, side);
            assert_eq!(world.cars[0].cop_damage(), damage, "retired bodies do not accumulate more health damage");
            assert!(!world.cars[0].take_wreck());
            world.cars[0].sims_mut().0.events.as_mut().unwrap().repair_lights();
            assert_eq!(world.cars[0].sims_mut().0.lamp_damage(), 0, "logical lamp reset only");
            assert!(world.cars[0].is_wrecked(), "lamp reset must not claim to repair cop health");
            eprintln!("{name}: {} partial={expected:.4} lethal={damage:.4} visual={levels:?}", DAMAGE_ZONES[zone]);
        }
        world.cars.clear();
        spawn(&mut world, name);
        assert_eq!(world.cars[0].cop_damage(), 0.0, "a newly spawned life has no retained health");
        assert_eq!(world.cars[0].sims_mut().0.lamp_damage(), 0);
        assert!(world.cars[0].destroy_by_breaker());
        assert_eq!(world.cars[0].sims_mut().0.events.as_ref().unwrap().source_levels(), &[6; 10]);
        assert_eq!(world.cars[0].sims_mut().0.lamp_damage(), u32::MAX);
    }
}

#[test]
#[ignore = "requires installed cop damage data"]
fn installed_exact_health_boundary_disables_lamps_without_emitting_a_strict_wreck_event() {
    let (_dir, data) = data();
    let mut renderer = NullBackend::new(crate::settings::test_caps::native());
    let mut model = cop_model(&mut renderer, "copmidsize", player());
    model.model.physics = data.car("copmidsize").unwrap();
    model.model.physics.geometry = None;
    let mut world = TrafficWorld::new(cop_road());
    world.add_cop_model(model);
    spawn(&mut world, "copmidsize");
    let mut player_physics = data.car("bmwm3gtr").unwrap();
    player_physics.geometry = None;
    let mut striker = CarSim::new(player_physics, [0.3; 4]);
    let mut hit = one_contact(&mut world, &mut striker, 1.01, true);
    let spec = world.cars[0].sims_mut().0.events.as_ref().unwrap().spec().clone();
    // Exact-boundary diagnostic changes only final callback force; the physical matrix above never does this.
    hit.force = spec.strength * spec.hit_points / spec.hit_point_scale[hit.zone()];
    world.cars[0].sims_mut().0.events.as_mut().unwrap().hit(hit);
    world.cars[0].on_hit(hit.force * 0.5, true, DriverClass::Human, hit.closing_speed, hit, 0.0);
    world.cars[0].sims_mut().0.events.as_mut().unwrap().tick(0.0);
    assert_eq!(world.cars[0].cop_damage(), 1.0);
    assert!(world.cars[0].is_wrecked());
    assert_eq!(world.cars[0].sims_mut().0.lamp_damage(), u32::MAX);
    assert!(!world.cars[0].take_wreck(), "native destruction event requires strictly greater than one");
    assert!(world.cars[0].sims_mut().0.events.as_ref().unwrap().source_levels().iter().all(|&level| level <= 2));
}
